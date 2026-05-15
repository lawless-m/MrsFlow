let r = try {
        List.Mode({1, 2, 2, 3, 3, 3, 4}),
        List.Mode({"a", "b", "a", "c", "a"}),
        try List.Mode({1, 2, 3}) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
