let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("i45WMjIwMlBwKk1JTy1R0lEysdAxNzXXMTQ0UIqNBQA=", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [Name = _t, Number = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Name", type text}, {"Number", Int64.Type}})
in
    #"Changed Type"