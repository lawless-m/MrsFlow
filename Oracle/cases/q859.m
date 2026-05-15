// List.Distinct with mixed primitive types — numbers/text/dates.
let r = try {
        List.Distinct({1, 1.0, 2, 2.0}),
        List.Distinct({"1", 1}),
        List.Distinct({true, false, true, true}),
        List.Distinct({null, null, null})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
