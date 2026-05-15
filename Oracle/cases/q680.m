// Infinity and NaN bases/exps.
let r = try {
        try Number.Power(Number.PositiveInfinity, 0) otherwise "err",
        try Number.Power(Number.PositiveInfinity, 1) otherwise "err",
        try Number.Power(Number.PositiveInfinity, -1) otherwise "err",
        try Number.Power(1, Number.PositiveInfinity) otherwise "err",
        try Number.Power(1, Number.NaN) otherwise "err",
        try Number.Power(Number.NaN, 0) otherwise "err",
        try Number.Power(Number.NaN, Number.NaN) otherwise "err",
        try Number.Power(Number.NegativeInfinity, 2) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
