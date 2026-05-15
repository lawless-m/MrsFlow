// Very small / very large bases.
let r = try {
        Number.Power(2, 53),
        Number.Power(2, 62),
        Number.Power(0.5, 10),
        Number.Power(0.5, 50),
        Number.Power(10, 100),
        Number.Power(10, -100),
        Number.Power(1.0000001, 1000000),
        Number.Power(0.9999999, 1000000)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
