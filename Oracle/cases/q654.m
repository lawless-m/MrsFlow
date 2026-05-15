let r = try {
        Text.Middle("hello world", 6, 5),
        Text.Middle("hello", 1, 3),
        Text.Middle("hello", 0, 5),
        Text.Middle("hello", 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
