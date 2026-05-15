let r = try {
        List.Average({1, 2, 3, 4, 5}),
        List.Average({10, 20, 30}),
        List.Average({}),
        List.Average({1, null, 3})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
