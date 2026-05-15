// List.Contains with culture-sensitive Comparer arg.
let r = try {
        List.Contains({"café", "résumé"}, "café"),
        List.Contains({"CAFÉ", "RÉSUMÉ"}, "café", Comparer.OrdinalIgnoreCase),
        List.Contains({"Straße"}, "STRASSE", Comparer.OrdinalIgnoreCase),
        List.Contains({"İstanbul"}, "ISTANBUL", Comparer.OrdinalIgnoreCase),
        List.Contains({"naïve"}, "NAÏVE", Comparer.OrdinalIgnoreCase)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
