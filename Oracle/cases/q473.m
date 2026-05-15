let r = try
        let
            d = Decimal.From(1.5),
            f = 2.0,
            sum = d + f
        in
            Value.Is(sum, Decimal.Type)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
