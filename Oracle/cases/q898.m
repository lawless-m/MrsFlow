// Single-element + null-only behaviour.
let r = try {
        List.Sum({1}),
        List.Average({1}),
        List.Median({5}),
        List.Mode({5}),
        List.StandardDeviation({5}),
        List.Sum({null}),
        List.Average({null}),
        List.Median({null})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
