let r = try {Comparer.Ordinal(1, 2), Comparer.Ordinal(2, 2), Comparer.Ordinal(3, 1), Comparer.Ordinal(null, 1)} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
