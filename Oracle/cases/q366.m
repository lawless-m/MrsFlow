let r = try {
        List.PositionOf({"a", "b", "c", "b", "a"}, "b"),
        List.PositionOf({"a", "b", "c", "b", "a"}, "z"),
        List.PositionOf({}, "x"),
        List.PositionOf({1, 2, 3}, 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
