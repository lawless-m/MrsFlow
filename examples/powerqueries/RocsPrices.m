// given a list of product codes and optional account code and period
// query the ROCS URL and get a CSV of results and turn them into a PQ table
// prefix all the column names with ROCS_
// left the number of columns dynamic in case I decide to return more columns

// EXAMPLE USE
// let
//    cells = CleanDescription(OrderFormCells("C:\Users\Matthew.Heath\OneDrive - Ramsden International\Ola\Easter\RI-Easter2024-Order Form.xlsx")),
//    rocsPriceTable = RocsPrice(cells[Code], null, null),
//    rieaster = Table.Join(cells, "Code", rocsPriceTable, "ROCS_Product", JoinKind.Inner)
// in
//    rieaster

let 
    rocs_price = (products as list, account as nullable text, period as nullable date) =>
        let
            ymd = if period is null then "" else Date.ToText(period, "yyyy-MM-dd"),
            account = if account is null then "" else account,
            productString = Text.Combine(products, ","),
            url = "https://dw.ramsden-international.com/cgi-bin/rocs_query.py?account=" & account & "&product=" & productString & "&period=" & ymd,
            raw = Csv.Document(Web.Contents(url), [Delimiter=",", Encoding=65001, QuoteStyle=QuoteStyle.None]),
            csv = Table.PromoteHeaders(raw, [PromoteAllScalars=true]),
            cols = Table.ColumnNames(csv),
            prefixed = List.Transform(cols, each {_, "ROCS_" & _}),
            renamed = Table.RenameColumns(csv, prefixed),
            addtypes = Table.TransformColumnTypes(Table.RenameColumns(csv, prefixed), {{"ROCS_Price", Currency.Type}})
        in
            addtypes
in
   rocs_price