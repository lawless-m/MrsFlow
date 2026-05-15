let r = try {
        try Number.Round(Number.NaN, 0, RoundingMode.AwayFromZero) otherwise "err",
        try Number.Round(Number.PositiveInfinity, 0, RoundingMode.AwayFromZero) otherwise "err",
        try Number.Round(Number.NegativeInfinity, 0, RoundingMode.AwayFromZero) otherwise "err",
        try Number.Round(Number.NaN, 0, RoundingMode.ToEven) otherwise "err",
        try Number.Round(Number.PositiveInfinity, 2, RoundingMode.ToEven) otherwise "err",
        try Number.Round(null, 0, RoundingMode.ToEven) otherwise "err",
        try Number.Round(1.5, null, RoundingMode.ToEven) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
