let r = try Text.FromBinary(Json.FromValue([
        empty_list = {},
        empty_rec = [],
        nullable = null,
        bools = {true, false, true}
    ]), TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
