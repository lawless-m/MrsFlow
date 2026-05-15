// List.Distinct preserves order — first occurrence wins.
let r = try {
        List.Distinct({3, 1, 2, 1, 3, 2}),
        List.Distinct({"c", "a", "b", "a", "c"}),
        List.Distinct({2, 1, 2, 3, 1})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
