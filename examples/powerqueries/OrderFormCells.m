let
    blankColumn = (columnNames as list) => 
        let
            numberedCols = List.Transform(List.Numbers(1, List.Count(columnNames)), each "Column" & Text.From(_)),
            comparedList = List.Zip({columnNames, numberedCols}),
            matchingPair = List.First(List.Select(comparedList, each _{0} = _{1}), {"", ""})
        in
            matchingPair{0},

    lookupRange = (rangeValue as text) as text =>  // OK I give in, dicts were impossible!
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

    activeCells = (workbook as text, column as text, value as text) =>
        let
            Source = Excel.Workbook(File.Contents(workbook), null, true),
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
                        
    OrderFormCells = (workbook as text) => activeCells(workbook, "Column2", "Code")
in OrderFormCells


// you can now use this in a new query e.g. = OrderFormCells("C:\Users\Matthew.Heath\HoldingArea\Ola\RI-Easter2024-Order Form.xlsx")
