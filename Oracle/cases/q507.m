let r = try {
        List.LastN({1, 2, 3, 4, 5}, 2),
        List.LastN({1, 2, 3, 4, 5}, 100),
        List.LastN({1, 2, 3, 4, 5}, 0),
        List.LastN({}, 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
