// List.Median edges.
let r = try {
        List.Median({}),
        List.Median({null, null}),
        List.Median({1}),
        List.Median({1, 2, 3}),
        List.Median({1, 2, 3, 4}),
        List.Median({null, 1, 2, null, 3})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
