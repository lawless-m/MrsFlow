let r = try {
        try Number.IntegerDivide(Number.NaN, 3) otherwise "err",
        try Number.IntegerDivide(3, Number.NaN) otherwise "err",
        try Number.IntegerDivide(Number.PositiveInfinity, 3) otherwise "err",
        try Number.IntegerDivide(3, Number.PositiveInfinity) otherwise "err",
        try Number.IntegerDivide(Number.NegativeInfinity, 3) otherwise "err",
        try Number.IntegerDivide(3, Number.NegativeInfinity) otherwise "err",
        try Number.IntegerDivide(Number.PositiveInfinity, Number.NegativeInfinity) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
