let r = try {
        Text.End("hello world", 5),
        Text.End("hi", 5),
        Text.End("", 3),
        Text.End("hello", 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
