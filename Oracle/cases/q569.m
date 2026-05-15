let r = try {
        Text.Length(Text.Lower("HELLO")),
        Text.Length(Text.Upper("hello")),
        Text.Lower("hello") = "hello",
        Text.Upper("HELLO") = "HELLO"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
