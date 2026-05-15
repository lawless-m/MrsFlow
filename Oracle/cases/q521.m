let r = try {
        List.Distinct({1, 2, 2, 3, 3, 3, 4}),
        List.Distinct({"a", "A", "b"}),
        List.Distinct({"a", "A", "b"}, Comparer.OrdinalIgnoreCase),
        List.Distinct({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
