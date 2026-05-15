// Fractional exp giving fractional/irrational result.
let r = try {
        Number.Power(2, 0.5),
        Number.Power(4, 0.5),
        Number.Power(8, 1/3),
        Number.Power(27, 1/3),
        Number.Power(2, 1/3),
        Number.Power(100, 0.5),
        Number.Power(0.25, 0.5),
        Number.Power(1, 0.5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
