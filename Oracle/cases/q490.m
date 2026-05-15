let r = try {
        try Text.Insert("hello", -1, "X") otherwise "err",
        try Text.Insert("hello", 10, "X") otherwise "err",
        try Text.Range("hello", 10) otherwise "err",
        try Text.Range("hello", 0, 100) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
