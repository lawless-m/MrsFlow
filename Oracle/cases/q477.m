let r = try
        let
            x = Number.RandomBetween(10, 20)
        in
            x >= 10 and x <= 20
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
