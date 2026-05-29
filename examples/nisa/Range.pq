let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("TY+7DsIwDEV/pcqIuvCGsTRC7AwMUYfQXlFLIUWphUS/HtcDZDzn2rq2c2ZpSlM974TIpimdWQmf0zAhKq4F655CQKe8Eb6+0JIPxB9V23nEMxLFh4qditSNSnuhmydOw4gibzrkQVZ5zH3eXUlwgQ/cK55+KNvABLX13+quSivSUkLLhZVM3WL+BOlNLeTQ5gs=", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Range Number" = _t, #"Range Name" = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Range Number", type text}, {"Range Name", type text}})
in
    #"Changed Type"