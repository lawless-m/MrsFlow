let r = try {
        List.FirstN({1, 2, 3, 4, 5}, 3),
        List.FirstN({1, 2, 3, 4, 5}, 0),
        List.FirstN({1, 2, 3, 4, 5}, 100),
        List.FirstN({}, 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
