let r = try Text.FromBinary(
    Json.FromValue({1, (x) => x*2, 3}),
    TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
