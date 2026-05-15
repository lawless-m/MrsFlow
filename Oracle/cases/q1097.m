// Type.Is with nullable annotations.
let r = try {
        Value.Is(null, type nullable number),
        Value.Is(42, type nullable number),
        Value.Is("text", type nullable number),
        Value.Is(null, type number)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
