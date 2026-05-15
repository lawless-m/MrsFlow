// Text.PositionOf with case comparer (built-in OrdinalIgnoreCase).
let r = try {
        Text.PositionOf("Hello hello HELLO", "hello"),
        Text.PositionOf("Hello hello HELLO", "hello", Occurrence.First, Comparer.OrdinalIgnoreCase),
        Text.PositionOf("Hello hello HELLO", "hello", Occurrence.All, Comparer.OrdinalIgnoreCase),
        Text.PositionOf("Hello hello HELLO", "HELLO", Occurrence.All)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
