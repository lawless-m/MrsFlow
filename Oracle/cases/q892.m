// List.Sum / Average on empty + all-null lists.
let r = try {
        List.Sum({}),
        List.Sum({null, null, null}),
        List.Sum({1}),
        List.Sum({1, null, 2}),
        List.Average({}),
        List.Average({null, null}),
        List.Average({1}),
        List.Average({1, null, 3})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
