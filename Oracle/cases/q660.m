let r = try {
        Number.Round(0, 0, RoundingMode.AwayFromZero),
        Number.Round(0, 0, RoundingMode.ToEven),
        Number.Round(0.0, 2, RoundingMode.Down),
        Number.Round(-0.0, 0, RoundingMode.AwayFromZero),
        Number.Round(1, 0, RoundingMode.ToEven),
        Number.Round(-1, 0, RoundingMode.ToEven),
        Number.Round(100, -2, RoundingMode.AwayFromZero),
        Number.Round(100, -3, RoundingMode.AwayFromZero)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
