let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("TY69CsIwFEZfJWSULv7rWBuKu4ND6BDbD3shJpAGhz69txfRjOecXL5Yq9e60vXrQQhZd5XVG+Y2xRlBcMvYjOQ9BuHdkilAtTEOk6j98sRlJApPEQcR6ZuPTHdHOcUJqlw6laGYPJe+3K45XOF8HgUvP+RrYIbY5m/lVqRhaSihz8pwE7did0N6Uw/+aPcB", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Range Number" = _t, #"Range Name" = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Range Number", type text}, {"Range Name", type text}})
in
    #"Changed Type"