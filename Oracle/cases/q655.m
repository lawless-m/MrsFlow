let r = try {
        Text.At("hello", 0),
        Text.At("hello", 4),
        try Text.At("hello", 10) otherwise "err",
        try Text.At("", 0) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
