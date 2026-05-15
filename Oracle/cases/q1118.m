// Text.StartsWith / Text.EndsWith default (Ordinal) + IgnoreCase.
let r = try {
        Text.StartsWith("café au lait", "café"),
        Text.StartsWith("Straße", "STRASS", Comparer.OrdinalIgnoreCase),
        Text.EndsWith("résumé", "umé"),
        Text.EndsWith("İSTANBUL", "anbul", Comparer.OrdinalIgnoreCase),
        Text.StartsWith("", ""),
        Text.EndsWith("hello", "")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
