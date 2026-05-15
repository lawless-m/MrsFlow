// DateTime accessor functions.
let dt = #datetime(2026, 6, 15, 10, 30, 45) in
let r = try {
        DateTime.Date(dt),
        DateTime.Time(dt),
        Date.Year(dt),
        Date.Month(dt),
        Date.Day(dt)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
