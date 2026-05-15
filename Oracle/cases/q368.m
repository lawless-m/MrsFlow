let r = try {
        List.PositionOfAny({"a", "b", "c"}, {"b", "z"}),
        List.PositionOfAny({"a", "b", "c"}, {"z", "y"}),
        List.PositionOfAny({"a", "b", "c"}, {})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
