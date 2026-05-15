let r = try
        let
            d = #duration(1, 2, 30, 45)
        in
            {Duration.Days(d), Duration.Hours(d), Duration.Minutes(d), Duration.Seconds(d), Duration.TotalSeconds(d)}
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
