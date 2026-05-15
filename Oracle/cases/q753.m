// Composition with other math functions.
let r = try {
        Number.Sign(Number.Sqrt(-1)),
        Number.Abs(Number.Sqrt(-1)),
        try Number.Sign(Number.Power(-2, 0.5)) otherwise "err",
        try Number.Abs(Number.Power(-2, 0.5)) otherwise "err",
        Number.Sign(Number.Round(0.001, 0)),
        Number.Abs(Number.Round(-0.001, 0)),
        Number.Sign(Number.Mod(-5, 3)),
        Number.Abs(Number.Mod(-5, 3))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
