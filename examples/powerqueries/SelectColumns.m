(tablename as text) =>
let
    src = Odbc.Query("DSN=Expormaster", "SELECT * FROM " & tablename & ";"),
    cols = Table.ColumnNames(src),
    str = "SELECT #(lf)" & Text.Combine(cols, ", #(lf)    ") & "FROM#(lf)    " & tablename & ";"
in
    str
