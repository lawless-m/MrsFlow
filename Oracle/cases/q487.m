let r = try {
        Text.Remove("hello world", "l"),
        Text.Remove("hello world", {"l", "o"}),
        Text.Remove("hello", "z"),
        Text.Remove("", "x")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
