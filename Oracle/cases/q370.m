let r = try List.PositionOf({"A", "b", "C"}, "a", Occurrence.First, Comparer.OrdinalIgnoreCase) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
