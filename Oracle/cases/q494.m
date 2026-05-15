let r = try Text.PositionOf("Hello World", "world", Occurrence.First, Comparer.OrdinalIgnoreCase) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
