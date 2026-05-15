let r = try {
        Date.AddQuarters(#date(2024, 11, 15), 1),
        Date.AddQuarters(#date(2024, 11, 15), 2),
        Date.AddWeeks(#date(2024, 12, 25), 2),
        Date.DayOfYear(#date(2024, 12, 31)),
        Date.DayOfYear(#date(2023, 12, 31))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
