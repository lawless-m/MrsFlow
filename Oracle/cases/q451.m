let r = try {
        Time.Hour(#time(14, 30, 45)),
        Time.Minute(#time(14, 30, 45)),
        Time.Second(#time(14, 30, 45)),
        Time.Hour(#time(0, 0, 0)),
        Time.Hour(#time(23, 59, 59))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
