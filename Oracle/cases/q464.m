let r = try {
        try Percentage.From("not a percent") otherwise "err",
        try Percentage.From("50") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
