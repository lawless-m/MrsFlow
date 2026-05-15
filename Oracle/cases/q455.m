let r = try
        let
            t1 = #time(14, 30, 0),
            t2 = #time(16, 45, 30),
            diff = t2 - t1
        in
            Duration.TotalMinutes(diff)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
