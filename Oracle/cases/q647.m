let r = try {
        List.RemoveMatchingItems({1, 2, 3, 1, 2}, {1, 2}),
        List.RemoveMatchingItems({"a", "b", "a", "c", "a"}, {"a"}),
        List.RemoveMatchingItems({1, 2, 3}, {})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
