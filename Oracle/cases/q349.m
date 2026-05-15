let r = try Binary.Length(Binary.FromText("", BinaryEncoding.Base64)) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
