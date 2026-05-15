let r = try
        let
            x = Number.Random()
        in
            x >= 0 and x < 1
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
