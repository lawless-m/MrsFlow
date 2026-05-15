// Non-integer / NaN / Inf / null inputs.
let r = try {
        try Number.BitwiseAnd(3.5, 5) otherwise "err",
        try Number.BitwiseAnd(5, 3.5) otherwise "err",
        try Number.BitwiseAnd(Number.NaN, 5) otherwise "err",
        try Number.BitwiseAnd(Number.PositiveInfinity, 5) otherwise "err",
        try Number.BitwiseAnd(null, 5) otherwise "err",
        try Number.BitwiseAnd(5, null) otherwise "err",
        try Number.BitwiseShiftLeft(1.5, 2) otherwise "err",
        try Number.BitwiseNot(3.5) otherwise "err",
        try Number.BitwiseNot(null) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
