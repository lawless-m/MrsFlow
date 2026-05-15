let r = try {
        Date.DayOfWeek(#date(2024, 6, 16)),
        Date.DayOfWeek(#date(2024, 6, 16), Day.Sunday),
        Date.DayOfWeek(#date(2024, 6, 16), Day.Monday),
        Date.DayOfWeekName(#date(2024, 6, 16))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
