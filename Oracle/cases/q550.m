let r = try
        let
            sin_sq = Number.Power(Number.Sin(0.5), 2),
            cos_sq = Number.Power(Number.Cos(0.5), 2),
            identity = sin_sq + cos_sq
        in
            Number.Abs(identity - 1) < 0.0000001
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
