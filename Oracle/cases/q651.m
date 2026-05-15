let r = try {
        Text.Length("hello"),
        Text.Length(""),
        Text.Length("a"),
        Text.Length("hello world")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
