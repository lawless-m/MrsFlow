let r = try {
        Number.Power(2, 10),
        Number.Power(10, -2),
        Number.Power(0, 0),
        Number.Power(-2, 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
