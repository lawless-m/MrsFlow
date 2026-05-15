// Time.From fractional days.
let r = try {
        Time.From(0.0),
        Time.From(0.25),
        Time.From(0.5),
        Time.From(0.75),
        Time.From(0.999),
        Time.From("10:30:00"),
        Time.From(#datetime(2026, 6, 15, 10, 30, 45))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
