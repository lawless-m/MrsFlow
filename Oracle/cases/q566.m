let r = try {
        Text.Lower("HELLO WORLD"),
        Text.Lower("Hello World"),
        Text.Lower(""),
        Text.Upper("hello world"),
        Text.Upper("aBcDeF")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
