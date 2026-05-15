let r = try {
        Text.PositionOf("hello world", "l"),
        Text.PositionOf("hello world", "world"),
        Text.PositionOf("hello world", "xyz"),
        Text.PositionOf("hello world", "")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
