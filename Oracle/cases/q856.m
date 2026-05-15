// List.Distinct default — first occurrence wins.
let r = try {
        List.Distinct({1, 2, 1, 3, 2}),
        List.Distinct({"a", "b", "a", "c", "b"}),
        List.Distinct({}),
        List.Distinct({1}),
        List.Distinct({null, 1, null, 2})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
