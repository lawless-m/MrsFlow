let r = try {
        Number.Sqrt(16),
        Number.Sqrt(2),
        Number.Sqrt(0),
        try Number.Sqrt(-1) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
