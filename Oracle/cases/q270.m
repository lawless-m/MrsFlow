let r = try Text.FromBinary(
    Json.FromValue([Name="x", Compute=(n) => n+1]),
    TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
