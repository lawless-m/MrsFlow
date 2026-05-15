let r = try {
        try Text.Lower("IZMIR", "tr-TR") otherwise "err",
        try Text.Lower("HELLO", "en-US") otherwise "err",
        try Text.Upper("istanbul", "tr-TR") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
