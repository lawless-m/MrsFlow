let r = try
        let
            v = Currency.From(123.456789)
        in
            v
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
