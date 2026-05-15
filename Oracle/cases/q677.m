// Normal cases — sanity baseline.
let r = try {
        Number.Power(2, 10),
        Number.Power(10, 3),
        Number.Power(3, 0),
        Number.Power(2, -3),
        Number.Power(1, 100),
        Number.Power(1, -100),
        Number.Power(0, 5),
        Number.Power(0, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
