// Type.Is wrong-type cases.
let r = try {
        Value.Is(42, type text),
        Value.Is("hello", type number),
        Value.Is(null, type number),
        Value.Is(true, type number),
        Value.Is({1, 2}, type record)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
