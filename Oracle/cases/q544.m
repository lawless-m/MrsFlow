let r = try {
        Number.Exp(1),
        Number.Ln(1),
        Number.Ln(Number.E),
        Number.Log10(100),
        Number.Log10(1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
