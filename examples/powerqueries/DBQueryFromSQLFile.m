(dsn as text, filepath as text) => 
    let
        func = Odbc.Query("dsn=" & dsn, Text.FromBinary(File.Contents(filepath)))
    in
        Value.ReplaceMetadata(func, [
            Documentation.Name = "DBQueryFromSQLFile",
            Documentation.Description = "Executes an SQL query from a file using ODBC connection",
            Documentation.LongDescription = "This function takes a DSN and file path, reads the SQL query from the file, and executes it against the specified ODBC data source.",
            Documentation.Category = "Database",
            Documentation.Examples = {
                [Description = "Basic usage", Code = "DBQueryFromSQLFile(""DSN={MyDSN}"", ""C:\query.sql"")", Result = "Table with query results"]
            }
        ])