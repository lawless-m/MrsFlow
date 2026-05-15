let r = try {
        Text.Trim(""),
        Text.Proper(""),
        Text.Trim("hello", "h"),
        Text.Trim("aaa", "a")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
