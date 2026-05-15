let r = try {
        try Number.Ln(0) otherwise "err",
        try Number.Ln(-1) otherwise "err",
        try Number.Log10(0) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
