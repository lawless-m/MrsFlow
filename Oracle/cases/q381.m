let r = try {Comparer.Ordinal("a", "b"), Comparer.Ordinal("b", "a"), Comparer.Ordinal("a", "a"), Comparer.Ordinal("A", "a")} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
