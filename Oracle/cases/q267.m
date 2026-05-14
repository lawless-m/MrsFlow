let r = try Text.FromBinary(
    Json.FromValue([a=1, f=(x) => x+1]),
    TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
