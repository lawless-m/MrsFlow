// Number.Sqrt Inf/NaN/null.
let r = try {
        try Number.Sqrt(Number.PositiveInfinity) otherwise "err",
        try Number.Sqrt(Number.NegativeInfinity) otherwise "err",
        try Number.Sqrt(Number.NaN) otherwise "err",
        try Number.Sqrt(null) otherwise "err",
        try Number.Sqrt(-0.0) otherwise "err",
        Number.Sqrt(0.0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
