(view as text) =>
    let 
        // Create the URL with parameters
        url = "http://rivsprod02:8000/cgi-bin/HTMLDataTable.exe?json=" & view,
        
        // Get JSON response
        response = Json.Document(Web.Contents(url)),
        
        // Extract the headers, types, and rows
        headers = response[headers],
        types = response[types],
        rows = response[rows],
        
        // Create a table from the rows array
        RowsTable = Table.FromRows(rows, headers)
        
        // Apply proper column types based on the types array
    in Table.TransformColumnTypes(
            RowsTable,
            List.Transform(
                List.Zip({headers, types}),
                (pair) => 
                    let
                        colName = pair{0},
                        colType = pair{1},
                        powerQueryType = 
                            if colType = "integer" then Int64.Type
                            else if colType = "number" then type number
                            else if colType = "boolean" then type logical
                            else if colType = "datetime" then type datetime
                            else type text
                    in
                        {colName, powerQueryType}
            )
        )
