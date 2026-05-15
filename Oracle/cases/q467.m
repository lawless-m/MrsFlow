let r = try {
        try Currency.From("123.45") otherwise "err",
        try Currency.From("$100.50") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
