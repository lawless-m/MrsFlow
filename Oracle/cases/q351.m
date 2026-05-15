let r = try Binary.ToText(Binary.FromList({0, 15, 16, 255}), BinaryEncoding.Hex) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
