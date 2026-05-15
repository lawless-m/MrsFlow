let r = try {
        Text.Contains("hello world", "world"),
        Text.Contains("hello world", "xyz"),
        Text.Contains("Hello World", "world", Comparer.OrdinalIgnoreCase),
        Text.StartsWith("hello world", "hello"),
        Text.EndsWith("hello world", "world")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
