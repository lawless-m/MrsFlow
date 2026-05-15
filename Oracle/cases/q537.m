let r = try Text.FromBinary(Json.FromValue([
        name = "alpha",
        nested = [a=1, b=[x=10, y=20]],
        items = {1, 2, 3}
    ]), TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
