// Number.Abs edge: NaN/Inf/null.
let r = try {
        try Number.Abs(Number.NaN) otherwise "err",
        try Number.Abs(Number.PositiveInfinity) otherwise "err",
        try Number.Abs(Number.NegativeInfinity) otherwise "err",
        try Number.Abs(null) otherwise "err",
        Number.Abs(1e308),
        Number.Abs(-1e308),
        Number.Abs(-1.7976931348623157e308),
        Number.Abs(-9007199254740992)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
