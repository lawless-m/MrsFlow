let r = try {
        Number.Round(List.StandardDeviation({2, 4, 4, 4, 5, 5, 7, 9}), 6),
        List.Min({3, 1, 4, 1, 5, 9}),
        List.Max({3, 1, 4, 1, 5, 9}),
        List.Count({1, 2, 3, 4, 5}),
        List.Count({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
