let r = try Binary.ToText(Binary.Range(Binary.FromText("48656c6c6f20576f726c64", BinaryEncoding.Hex), 6, 5), BinaryEncoding.Hex) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
