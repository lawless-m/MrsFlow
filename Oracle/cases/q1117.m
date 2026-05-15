// Text.Contains with Comparer.OrdinalIgnoreCase — case-fold, no Unicode normalisation.
let r = try {
        Text.Contains("CAFÉ", "café", Comparer.OrdinalIgnoreCase),
        Text.Contains("STRASSE", "strasse", Comparer.OrdinalIgnoreCase),
        Text.Contains("İstanbul", "ISTANBUL", Comparer.OrdinalIgnoreCase),
        Text.Contains("Hello World", "WORLD", Comparer.OrdinalIgnoreCase),
        Text.Contains("naïve", "NAÏVE", Comparer.OrdinalIgnoreCase)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
