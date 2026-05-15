// Text.Lower/Upper with empty / null / non-letter / mixed-case input.
let r = try {
        Text.Upper(""),
        Text.Lower(""),
        Text.Upper("abc123!@#"),
        Text.Lower("ABC123!@#"),
        Text.Upper("aBcDe"),
        Text.Lower("aBcDe")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
