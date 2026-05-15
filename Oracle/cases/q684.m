// Number.Sqrt across normal & boundary values.
let r = try {
        Number.Sqrt(0),
        Number.Sqrt(1),
        Number.Sqrt(4),
        Number.Sqrt(2),
        Number.Sqrt(0.25),
        Number.Sqrt(1e100),
        Number.Sqrt(1e-100),
        try Number.Sqrt(-1) otherwise "err",
        try Number.Sqrt(-0.0001) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
