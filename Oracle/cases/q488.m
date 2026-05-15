let r = try {
        Text.RemoveRange("hello world", 5),
        Text.RemoveRange("hello world", 5, 1),
        Text.RemoveRange("hello", 0, 5),
        Text.RemoveRange("hello", 2, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
