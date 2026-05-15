// Null propagation and overflow.
let r = try {
        try Number.Power(null, 5) otherwise "err",
        try Number.Power(5, null) otherwise "err",
        try Number.Power(null, null) otherwise "err",
        try Number.Power(null, 0) otherwise "err",
        Number.Power(2, 1024),
        Number.Power(2, 1023),
        Number.Power(2, -1024),
        Number.Power(2, -1074)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
