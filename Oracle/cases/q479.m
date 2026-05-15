let r = try
        let
            x = Number.RandomBetween(5, 5)
        in
            x
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
