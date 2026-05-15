let r = try
        let
            d = #date(2024, 6, 30) - #date(2024, 6, 15)
        in
            {Duration.TotalDays(d), Duration.TotalHours(d), Duration.TotalMinutes(d), Duration.TotalSeconds(d)}
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
