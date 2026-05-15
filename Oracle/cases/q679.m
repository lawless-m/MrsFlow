// 0^0 (indeterminate), 0^negative (infinity in IEEE).
let r = try {
        Number.Power(0, 0),
        try Number.Power(0, -1) otherwise "err",
        try Number.Power(0, -0.5) otherwise "err",
        try Number.Power(0, -2) otherwise "err",
        Number.Power(0, 0.5),
        Number.Power(0, 1.5),
        Number.Power(-0, 0),
        Number.Power(-0, 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
