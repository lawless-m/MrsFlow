let r = try Text.FromBinary(Json.FromValue([a=1, b=2]), TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
