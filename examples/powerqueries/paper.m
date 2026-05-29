// eval_file("C:\Users\Matthew.Heath\Git\PowerQueries\paper.m")
// eval_url("https://dw.ramsden-international.com/gogs/Ramsden-International/PowerQueries/raw/master/paper.m")
(dir as text, cust as text) =>
let

    suffix = ".xlsx",
    xl = FilterFiles(dir, suffix, "(" & cust & ")"),
    cells = OrderFormCells(dir & "/" & xl{0}[Name]),
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