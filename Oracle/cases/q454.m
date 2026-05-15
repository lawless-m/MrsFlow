let r = try
        let
            t = #time(14, 30, 45),
            d = #duration(0, 1, 30, 0),
            sum = t + d
        in
            sum
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
