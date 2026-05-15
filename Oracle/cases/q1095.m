// Type.Is across primitive types.
let r = try {
        Value.Is(42, type number),
        Value.Is("hello", type text),
        Value.Is(true, type logical),
        Value.Is(null, type null),
        Value.Is(#date(2026, 1, 1), type date),
        Value.Is({1, 2}, type list),
        Value.Is([a=1], type record)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
