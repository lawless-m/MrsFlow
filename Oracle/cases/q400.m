let r = try {
        Value.Is(42, type number),
        Value.Is("hi", type number),
        Value.Is(null, type number),
        Value.Is(null, type nullable number),
        Value.Is({1, 2}, type list)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
