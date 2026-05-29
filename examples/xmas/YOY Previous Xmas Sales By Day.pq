let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("pZRNlhoxDITvwloLyfo/C29Ol5vkZBGW6DADDCSz4dHd9ZXlkuzz+UQnOP3+hV9+588HnE/rlYBfCeSVQJ8LHMhaZO+I/B1RPBdFAkqrcr9YAmi3O1wBiI/01N9pGST1EqQ44lLiLZcCxtpcd4BFQcjuOHoOrjuQ8S+49CnI/wvKv4CkBkjZpH5L8hfSA9KiyVarCaxotd6Qgp/Ii1F9brKHgUB1QWT72C37eVU1BV7U7DQQUgQSOzAngfCp4Rt2JgfQeQew2ZWgM0iWVc9+f8euSRKWBJjEK/ayHZVOatEP2PWADZfqlkwS6HWC4tixhQMHNz0DBeQKPl5UbG0scp4cGWxX0mPFCzKl+ZkrSCZg1amF3IDlun6kg6EdMwLCCOLj0MzWg+Y1Ramag2Oeova//HCoBnEW0BO6ej6iaC3mWpFU1REzsTud3OfuGtdl3mxS6HnLUtUHxjcdrKKporZD/NghHzgIEF+uMR8HT4SFdJyZctA6XJ3DMAlildj04vKZgDTHzzMg7JikuiEsql3dC6ZxuJwRd5lcuVbV8LmLkXx6Ua3cR+k4lgZuWHeOt9t66Ba1IAfNE8aa22FemPoOaTvwA4dKqC5qxrw61NxwHLOxr3VIL4ePPw==", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Day No." = _t, #"2022 Sales" = _t, #"2023 Sales" = _t, #"2024 Sales" = _t, #"Deadline Day Target" = _t, #"2025 Sales" = _t]),
    #"Reordered Columns" = Table.ReorderColumns(Source,{"Day No.", "2022 Sales", "2023 Sales", "2024 Sales", "2025 Sales", "Deadline Day Target"}),
    #"Changed Type" = Table.TransformColumnTypes(#"Reordered Columns",{{"Day No.", Int64.Type}, {"2022 Sales", Currency.Type}, {"2023 Sales", Currency.Type}, {"2024 Sales", Currency.Type}, {"Deadline Day Target", Currency.Type}, {"2025 Sales", Currency.Type}}),
    #"Added Custom" = Table.AddColumn(#"Changed Type", "Custom", each [2025 Sales] - [2024 Sales]),
    #"Renamed Columns" = Table.RenameColumns(#"Added Custom",{{"Custom", "Gap 24 vs 25 Sales"}}),
    #"Added Custom1" = Table.AddColumn(#"Renamed Columns", "Custom", each 0),
    #"Renamed Columns1" = Table.RenameColumns(#"Added Custom1",{{"Custom", "Constant Zero"}})
in
    #"Renamed Columns1"