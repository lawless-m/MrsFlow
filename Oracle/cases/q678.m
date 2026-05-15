// Negative base × fractional exp — the classic NaN territory.
let r = try {
        try Number.Power(-2, 0.5) otherwise "err",
        try Number.Power(-2, 1.5) otherwise "err",
        try Number.Power(-1, 0.5) otherwise "err",
        try Number.Power(-1, 0.3) otherwise "err",
        Number.Power(-2, 2),
        Number.Power(-2, 3),
        Number.Power(-2, -2),
        Number.Power(-2, -3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
