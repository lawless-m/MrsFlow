let r = try {
        List.Skip({1, 2, 3, 4, 5}, 2),
        List.Skip({1, 2, 3, 4, 5}, 0),
        List.Skip({1, 2, 3, 4, 5}, 100),
        List.Skip({}, 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
