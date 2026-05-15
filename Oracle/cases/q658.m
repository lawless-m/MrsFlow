let r = try {
        Number.Round(1.25, 1, RoundingMode.AwayFromZero),
        Number.Round(1.25, 1, RoundingMode.ToEven),
        Number.Round(1.25, 1, RoundingMode.Down),
        Number.Round(1.25, 1, RoundingMode.Up),
        Number.Round(1.25, 1, RoundingMode.TowardZero),
        Number.Round(-1.25, 1, RoundingMode.Down),
        Number.Round(-1.25, 1, RoundingMode.Up),
        Number.Round(-1.25, 1, RoundingMode.TowardZero)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
