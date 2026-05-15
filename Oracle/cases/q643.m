let r = try {
        List.Median({1, 2, 3, 4, 5}),
        List.Median({1, 2, 3, 4}),
        List.Median({5, 1, 4, 2, 3}),
        List.Median({1})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
