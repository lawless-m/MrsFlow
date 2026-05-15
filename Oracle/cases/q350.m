let r = try Binary.ToText(Text.ToBinary("Hello", TextEncoding.Utf8), BinaryEncoding.Hex) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
