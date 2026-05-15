let r = try {
        List.Transform({1, 2, 3}, each _ * 2),
        List.Transform({1, 2, 3}, (x) => x * 2),
        List.Transform({1, 2, 3}, Number.Sqrt),
        List.Transform({}, each _)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
