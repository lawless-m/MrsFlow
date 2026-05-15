let r = try {
        Date.AddYears(#date(2024, 2, 29), 1),
        Date.AddYears(#date(2024, 2, 29), 4),
        Date.AddYears(#date(2020, 2, 29), -100),
        Date.AddMonths(#date(2024, 1, 31), 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
