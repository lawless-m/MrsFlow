let r = try {
        List.Sum({1, 2, 3, 4, 5}),
        List.Sum({}),
        List.Sum({1.5, 2.5, 3.0}),
        List.Sum({1, null, 3})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
