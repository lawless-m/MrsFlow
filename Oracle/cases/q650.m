let r = try {
        List.Sort({3, 1, 4, 1, 5, 9, 2, 6}),
        List.Sort({"banana", "apple", "cherry"}),
        List.Sort({}),
        List.Sort({3, 1, 4, 1, 5, 9, 2, 6}, Order.Descending)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
