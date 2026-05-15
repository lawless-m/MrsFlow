let r = try {
        List.Reverse({1, 2, 3, 4, 5}),
        List.Reverse({}),
        List.Reverse({"a"}),
        List.Reverse(List.Reverse({1, 2, 3}))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
