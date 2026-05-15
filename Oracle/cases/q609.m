let r = try
        let
            negDur = #duration(-1, 0, 0, 0),
            dur2 = #duration(0, 25, 0, 0)
        in
            {Duration.TotalDays(negDur), Duration.TotalHours(dur2), Duration.Days(dur2), Duration.Hours(dur2)}
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
