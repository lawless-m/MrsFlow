let r = try
        let
            naninf = Number.NaN,
            pinf = Number.PositiveInfinity
        in
            {
                try Number.Mod(naninf, 1) otherwise "err",
                try Number.Mod(1, pinf) otherwise "err",
                try Number.IntegerDivide(pinf, 1) otherwise "err"
            }
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
