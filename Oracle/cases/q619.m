let r = try {
        Date.DaysInMonth(#date(2024, 2, 1)),
        Date.DaysInMonth(#date(2023, 2, 1)),
        Date.DaysInMonth(#date(2024, 1, 1)),
        Date.DaysInMonth(#date(2024, 4, 1))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
