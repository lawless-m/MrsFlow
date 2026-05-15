let r = try {
        List.FirstN({1, 2, 3, 4, 5}, each _ < 4),
        List.FirstN({5, 4, 3, 2, 1}, each _ < 4),
        List.RemoveFirstN({1, 2, 3, 4, 5}, each _ < 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
