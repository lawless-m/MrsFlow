let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("Tc49DsIwDAXgq0QZURf+YSyNuAADQ9QhpBZYSlPkBiQ4PY7F4PF7ftKz93ZpG9uON4RcbN94u2KfafpCFq6Z3QNTgkG8YV+eEDEkLB+JtrUSChDmuwQ7CWiYRXvWNWChaQajlw76oCaPOtfbbd1+jUFw+sOIOn1yDIcEsRgHIUm2qAWgN0bgt/of", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Range Number" = _t, #"Range Name" = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Range Number", type text}, {"Range Name", type text}})
in
    #"Changed Type"