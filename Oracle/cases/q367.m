let r = try List.PositionOf({"a", "b", "c", "b", "a"}, "b", Occurrence.All) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
