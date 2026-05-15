// Number.Exp Inf/NaN/null + round-trip relations.
let r = try {
        try Number.Exp(Number.PositiveInfinity) otherwise "err",
        try Number.Exp(Number.NegativeInfinity) otherwise "err",
        try Number.Exp(Number.NaN) otherwise "err",
        try Number.Exp(null) otherwise "err",
        // Ln/Exp round-trip
        Number.Ln(Number.Exp(2)),
        Number.Exp(Number.Ln(2)),
        // Sqrt/Power round-trip
        Number.Sqrt(Number.Power(2, 4)),
        Number.Power(Number.Sqrt(2), 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
