let r = try {
        Number.Round(0.5, 0, RoundingMode.AwayFromZero),
        Number.Round(1.5, 0, RoundingMode.AwayFromZero),
        Number.Round(2.5, 0, RoundingMode.AwayFromZero),
        Number.Round(-0.5, 0, RoundingMode.AwayFromZero),
        Number.Round(-1.5, 0, RoundingMode.AwayFromZero),
        Number.Round(-2.5, 0, RoundingMode.AwayFromZero)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
