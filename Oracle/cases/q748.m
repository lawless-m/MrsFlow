// Number.Sign edge: NaN/Inf/null.
let r = try {
        try Number.Sign(Number.NaN) otherwise "err",
        try Number.Sign(Number.PositiveInfinity) otherwise "err",
        try Number.Sign(Number.NegativeInfinity) otherwise "err",
        try Number.Sign(null) otherwise "err",
        Number.Sign(1234567890),
        Number.Sign(-1234567890),
        Number.Sign(1e308),
        Number.Sign(-1e308)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
