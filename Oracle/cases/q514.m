let r = try {
        List.Numbers(1, 5),
        List.Numbers(0, 10, 2),
        List.Numbers(10, 5, -1),
        List.Numbers(1, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
