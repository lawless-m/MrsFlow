let r = try {
        List.RemoveFirstN({1, 2, 3, 4, 5}, 2),
        List.RemoveFirstN({1, 2, 3, 4, 5}, 100),
        List.RemoveLastN({1, 2, 3, 4, 5}, 2),
        List.RemoveLastN({1, 2, 3, 4, 5}, 100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
