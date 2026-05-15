let r = try {
        List.Range({1, 2, 3, 4, 5}, 1, 3),
        List.Range({1, 2, 3, 4, 5}, 0, 5),
        List.Range({1, 2, 3, 4, 5}, 2, 0),
        List.Range({1, 2, 3, 4, 5}, 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
