let r = try {
        List.Skip({1, 2, 3, 4, 5}, each _ < 3),
        List.Skip({5, 4, 3, 2, 1}, each _ < 3),
        List.Skip({1, 2, 3}, each _ < 100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
