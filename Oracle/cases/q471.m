let r = try
        let
            a = Decimal.From(0.1),
            b = Decimal.From(0.2),
            sum = a + b
        in
            sum
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
