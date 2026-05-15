let r = try Binary.ToText(Binary.FromText("48656c6c6f", BinaryEncoding.Hex), BinaryEncoding.Base64) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
