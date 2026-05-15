let r = try {
        try Number.Mod(Number.NaN, 3) otherwise "err",
        try Number.Mod(3, Number.NaN) otherwise "err",
        try Number.Mod(Number.PositiveInfinity, 3) otherwise "err",
        try Number.Mod(3, Number.PositiveInfinity) otherwise "err",
        try Number.Mod(Number.NegativeInfinity, 3) otherwise "err",
        try Number.Mod(3, Number.NegativeInfinity) otherwise "err",
        try Number.Mod(Number.PositiveInfinity, Number.PositiveInfinity) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
