let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("VdBJCsMwDIXhuxi6E0EeJNtnCVl0Hm+Vm+RkbYNeYi+M4edDxhpHt8wxETP/jyPHQ+TD715mz8Ha0U1kMDdQAHMkWdsJULuJajB4tXYGLB3MgAp4AaxUG1gNRqnk13Y1mEIz0Q9sMAW0G6B20BsUxtP3HWoDsR5lobK2h0Hpn04GcyjWnjuUBmI9JaG9AKWbWAzWivY2qNzAsP3aU9jqx03TFw==", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Sales " = _t, Rebate = _t, Value = _t, #"Rebate Slab" = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Sales ", Currency.Type}, {"Rebate", Percentage.Type}, {"Value", Currency.Type}})
in
    #"Changed Type"