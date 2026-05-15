let r = try {
        Value.Compare("a", "A", Comparer.OrdinalIgnoreCase),
        Value.Compare("A", "a", Comparer.Ordinal),
        Value.Compare(#date(2024, 1, 1), #date(2024, 6, 1))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
