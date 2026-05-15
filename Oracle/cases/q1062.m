// Date.AddDays — year rollover, leap span.
let r = try {
        Date.AddDays(#date(2026, 12, 31), 1),
        Date.AddDays(#date(2026, 1, 1), -1),
        Date.AddDays(#date(2024, 2, 28), 1),
        Date.AddDays(#date(2024, 2, 28), 2),
        Date.AddDays(#date(2025, 2, 28), 1),
        Date.AddDays(#date(2025, 2, 28), 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
