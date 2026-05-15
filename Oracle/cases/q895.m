// List.StandardDeviation edges.
let r = try {
        List.StandardDeviation({}),
        List.StandardDeviation({1}),
        List.StandardDeviation({1, 1, 1}),
        List.StandardDeviation({1, 2, 3, 4, 5}),
        List.StandardDeviation({null, 1, 2, null, 3, 4, 5})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
