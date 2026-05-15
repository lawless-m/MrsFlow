let r = try {
        Number.Round(1234.5, 0, RoundingMode.Up),
        Number.Round(1234.5, 0, RoundingMode.Down),
        Number.Round(-1234.5, 0, RoundingMode.Up),
        Number.Round(-1234.5, 0, RoundingMode.Down),
        Number.Round(0.5, 0, RoundingMode.TowardZero),
        Number.Round(-0.5, 0, RoundingMode.TowardZero),
        Number.Round(2.675, 2, RoundingMode.ToEven),
        Number.Round(2.675, 2, RoundingMode.AwayFromZero)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
