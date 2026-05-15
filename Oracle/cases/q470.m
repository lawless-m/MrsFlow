let r = try
        let
            a = Currency.From(10.5),
            b = Currency.From(2.25),
            sum = a + b
        in
            sum
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
