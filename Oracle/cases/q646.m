let r = try {
        List.RemoveItems({1, 2, 3, 4, 5}, {2, 4}),
        List.RemoveItems({1, 2, 3}, {}),
        List.RemoveItems({"a", "b", "c"}, {"x"}),
        List.RemoveItems({}, {1, 2})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
