let
    blankColumn = (columnNames as list) => 
        let
            numberedCols = List.Transform(List.Numbers(1, List.Count(columnNames)), each "Column" & Text.From(_)),
            comparedList = List.Zip({columnNames, numberedCols}),
            matchingPair = List.First(List.Select(comparedList, each _{0} = _{1}), {"", ""})
        in
            matchingPair{0},

    lookupRange = (rangeValue as text) as text =>  //  dicts were impossible!
              if rangeValue = "1" then "Ambient"
         else if rangeValue = "2" then "Frozen"
         else if rangeValue = "3" then "Chilled"
         else if rangeValue = "4" then "Fine Foods"
         else if rangeValue = "9" then "Health"
         else "Unknown Range",

    firstRow = (rows as table, column as text, value as nullable text) =>
        let
            indexed = Table.AddIndexColumn(rows, "Index", 0, 1, Int64.Type),
            FilteredRows = Table.SelectRows(indexed, each Record.Field(_, column) = value)
        in
            if Table.IsEmpty(FilteredRows) then null else FilteredRows{0}[Index],

    cleanDesc = (d as text) =>
        let 
            dd = Text.Replace(d, " .", ""),
            dc = Text.Replace(dd, "★", ""),
            db = Text.Replace(dc, "📅", ""),
            da = Text.Replace(db, "🏅", "")
        in Text.Trim(da),

          // Function to navigate through the folder tree
    GetFolderContents = (url as text, folderPath as text) as table =>
        let
            // Get the root contents
            Source = SharePoint.Contents(url, [ApiVersion = 15]),

            // Escape the folder path because Sharepoint will give it you you unescaped
            EscapedFolderPath = Uri.EscapeDataString(folderPath),

            // Split the folder path on "/" for a list of folder names
            PathList = Text.Split(EscapedFolderPath, "%2F"), // %2F is the encoded value for "/"

            // Function to unescape folder names because ' will break things even though it's valid
            UnescapeFolderName = (folderName as text) as text =>
                Text.Replace(Text.Replace(folderName, "%20", " "), "%27", "'"),

            // Recursive function to navigate through the folder path
            NavigateFolders = (currentFolder as table, path as list) as table =>
                if List.IsEmpty(path) then
                    currentFolder
                else
                    let
                        nextFolderName = UnescapeFolderName(List.First(path)),
                        nextFolder = currentFolder{[Name=nextFolderName]}[Content],
                        remainingPath = List.RemoveFirstN(path, 1)
                    in
                        @NavigateFolders(nextFolder, remainingPath),
                        
			// recursively get the contents	
            FinalFolderContents = NavigateFolders(Source, PathList)
        in
            FinalFolderContents,

    // Function to convert folder contents to a table
    cont2tab = (content as table) as table =>
        let
            CreateRecord = (row) =>
                [Name = row[Name], Content = row[Content], FolderPath = row[Folder Path]],

            // Create a list of records from the content table
            records = List.Transform(Table.ToRecords(content), each CreateRecord(_)),
            tab = Table.FromRecords(records)
        in
            Table.SelectRows(tab, each Text.EndsWith([Name], ".xlsx")),

    cont2fileList = (conts) as table =>
        let
            FolderContentsList = Table.ToRecords(conts),
            AllTables = List.Transform(FolderContentsList, each cont2tab(_[Content]))            
        in
            Table.Combine(AllTables),

    orderFormBinary = (acc as text) as table =>
        let
            url = "https://ramsdenint.sharepoint.com/sites/CustomerServices/",

            baset = cont2fileList(GetFolderContents(url, "7 Sales/Easter 2025/Order Forms")),
            eaas =  cont2fileList(GetFolderContents(url, "7 Sales/Easter 2025/Order Forms/EAAs")),

            fileList = Table.Combine({baset, eaas}),

            file = Table.SelectRows(fileList, each Text.Contains([Name], acc))
        in
            file,

    activeCells = (acc_code as text, column as text, value as text) =>
        let
            fileTable = orderFormBinary(acc_code),
            fileBinary = fileTable{0}[Content],
            Source = Excel.Workbook(fileBinary, null, true),
            grid = Source{[Item="OrderForm",Kind="Sheet"]}[Data],
            skiprows = firstRow(grid, column, value),
            skipped = Table.Skip(grid, skiprows),
            endrow = firstRow(skipped, column, null),
            rows = Table.PromoteHeaders(Table.FirstN(skipped, endrow), [PromoteAllScalars=true]),
            blnk = blankColumn(Table.ColumnNames(rows)),
            moveRange = Table.ReorderColumns(rows, {"Range"} & List.RemoveItems(Table.ColumnNames(rows), {"Range"})),
            doLookupRange = Table.TransformColumns(moveRange, {{"Range", each lookupRange(Text.From(_)), type text}}),
            columnNames = Table.ColumnNames(doLookupRange),
            selectColumns = List.FirstN(columnNames, List.PositionOf(columnNames, blnk)),
            reduceCols = Table.SelectColumns(doLookupRange, selectColumns),
            renameBarcode = Table.RenameColumns(reduceCols,{{"Inner Barcode#(lf)(CLICK FOR IMAGE)", "Inner Barcode"}}),
            realColumnNames = Table.ColumnNames(renameBarcode),
            columnTypes = List.Transform(realColumnNames, each {_, type text}),
            typedCols = Table.TransformColumnTypes(renameBarcode, columnTypes),
            hatedTable = Table.TransformColumns(typedCols, {{"Description", each cleanDesc(_), type text}})
        in
            hatedTable,
                        
    OrderFormCells = (acc_code as text) => activeCells(acc_code, "Column2", "Code"),

    paper = (acc_code as text) =>
        let
            cells = OrderFormCells(acc_code),
            dateCols = List.Select(Table.ColumnNames(perCase), each Text.Contains(_, "/20")),
            perCase = Table.TransformColumns(cells, {{"Units Per Case", each _ & " Per Case", type text}}),
            priceColumn = List.Select(Table.ColumnNames(perCase), each Text.StartsWith(_, "Price")){0},
            boughtIn = List.Select(Table.ColumnNames(perCase), each Text.StartsWith(_, "Bought")){0},
            drp = Table.RemoveColumns(perCase, {"Category", "Page/Item", "Inner Barcode", boughtIn}),
            columnNames = Table.ColumnNames(drp),
            
            paperTable = #table(
                {"Id", "Name", "Units", "Price option", "Release date", "Range", "Image URL"},
                {}
            ),

            getPrice = (row) => 
                let
                    p = Record.Field(row, priceColumn),
                    q = if p = null then ""
                        else if p = "call" then "" 
                        else Number.Round(Number.From(p), 2)
                in q,

            CreatePaperTableRows = (row) =>
                List.Combine(
                    List.Transform(
                        dateCols,
                        (dateCol) => if Record.Field(row, dateCol) = null then {[
                            Id = row[Code], 
                            Name = row[Description], 
                            Units = row[#"Units Per Case"], 
                            #"Release date" = Date.FromText(Text.Replace(dateCol, "/", "-")), 
                            #"Price option" = getPrice(row),
                            Range = row[Range],
                            #"Image URL" = "https://ramsden-int.files.svdcdn.com/production/Easter25_repo/" & row[Code] & ".jpg"
                        ]} else {}
                    )
                ),
            
            newRows = List.Combine(Table.TransformRows(drp, CreatePaperTableRows)),
            updatedPaperTable = Table.InsertRows(paperTable, 0, newRows),
            named = Table.TransformColumnTypes(updatedPaperTable,{{"Id", type text}, {"Name", type text}, {"Units", type text}, {"Release date", type date}, {"Price option", type any}, {"Range", type text}, {"Image URL", type text}})
            
        in
            named

in paper

// we used this as a single query and edited the function call on each worksheet to the appropriate account code
// defined eval_url (copied it from the repro - https://dw.ramsden-international.com/gogs/Ramsden-International/PowerQueries/src/master/evalUrl.m
// defined PaperSharepoint as = eval_url("https://dw.ramsden-international.com/gogs/Ramsden-International/PowerQueries/raw/master/PaperSharepoint.m")
// Invoked it e.g. = PaperSharepoint("992728")
