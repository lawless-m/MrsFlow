// NaN/Inf in numeric aggregates.
let r = try {
        List.Sum({1, 2, 1/0, 3}),
        List.Sum({1, -1/0, 1/0}),
        List.Sum({1, 0/0, 2}),
        List.Average({1, 1/0}),
        List.Max({1, 1/0, 3}),
        List.Min({1, -1/0, 3})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
