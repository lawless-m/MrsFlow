let r = try {
        List.Repeat({1, 2, 3}, 3),
        List.Repeat({1, 2, 3}, 0),
        List.Repeat({}, 5),
        List.Repeat({"x"}, 5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
