// Value.Type — extracts a type value. Raw type values aren't
// serialisable to JSON in either engine, so test the round-trip
// predicate (Value.Is(v, Value.Type(v))) holds.
let r = try {
        Value.Is(42, Value.Type(42)),
        Value.Is("hello", Value.Type("hello")),
        Value.Is(true, Value.Type(true)),
        Value.Is(null, Value.Type(null)),
        Value.Is({1, 2}, Value.Type({1, 2})),
        Value.Is([a=1], Value.Type([a=1]))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
