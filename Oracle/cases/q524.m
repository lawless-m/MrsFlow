let r = try {
        List.Difference({1, 2, 3, 4, 5}, {2, 4}),
        List.Difference({1, 2, 3}, {}),
        List.Difference({}, {1, 2, 3}),
        List.Difference({"a", "B", "c"}, {"A", "C"}, Comparer.OrdinalIgnoreCase)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
