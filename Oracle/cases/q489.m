let r = try {
        Text.Range("hello world", 6),
        Text.Range("hello world", 0, 5),
        Text.Range("hello", 0, 0),
        Text.Range("hello", 5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
