// Number.Ln Inf/NaN/null + Number.Log10.
let r = try {
        try Number.Ln(Number.PositiveInfinity) otherwise "err",
        try Number.Ln(Number.NegativeInfinity) otherwise "err",
        try Number.Ln(Number.NaN) otherwise "err",
        try Number.Ln(null) otherwise "err",
        Number.Log10(1),
        Number.Log10(10),
        Number.Log10(100),
        Number.Log10(0.1),
        try Number.Log10(0) otherwise "err",
        try Number.Log10(-1) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
