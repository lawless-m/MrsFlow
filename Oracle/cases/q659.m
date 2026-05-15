let r = try {
        Number.Round(1234.5678, -2, RoundingMode.AwayFromZero),
        Number.Round(1234.5678, -1, RoundingMode.AwayFromZero),
        Number.Round(1234.5678, 0, RoundingMode.AwayFromZero),
        Number.Round(1234.5678, 1, RoundingMode.AwayFromZero),
        Number.Round(1234.5678, 2, RoundingMode.AwayFromZero),
        Number.Round(1234.5678, 6, RoundingMode.AwayFromZero),
        Number.Round(0.000123456, 4, RoundingMode.AwayFromZero),
        Number.Round(0.000123456, 8, RoundingMode.AwayFromZero)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
