let r = try {
        Text.Start("hello world", 5),
        Text.Start("hi", 5),
        Text.Start("", 3),
        Text.Start("hello", 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
