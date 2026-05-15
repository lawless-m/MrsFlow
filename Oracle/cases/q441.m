let r = try {
        Date.WeekOfMonth(#date(2024, 1, 1)),
        Date.WeekOfMonth(#date(2024, 1, 15)),
        Date.WeekOfMonth(#date(2024, 1, 31)),
        Date.WeekOfMonth(#date(2024, 12, 31))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
