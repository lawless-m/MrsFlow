// Text.PositionOf with culture-sensitive chars + comparer arg.
let r = try {
        Text.PositionOf("café résumé", "é"),
        Text.PositionOf("Straße", "ß"),
        Text.PositionOf("CAFÉ", "é", 0, Comparer.OrdinalIgnoreCase),
        Text.PositionOf("İstanbul", "i", 0, Comparer.OrdinalIgnoreCase),
        Text.PositionOf("hello", "x")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
