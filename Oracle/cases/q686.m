// Number.Ln across normal & boundary values.
let r = try {
        Number.Ln(1),
        try Number.Ln(0) otherwise "err",
        try Number.Ln(-1) otherwise "err",
        try Number.Ln(-0.0001) otherwise "err",
        Number.Ln(2.718281828459045),
        Number.Ln(10),
        Number.Ln(0.5),
        Number.Ln(1e100),
        Number.Ln(1e-100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
