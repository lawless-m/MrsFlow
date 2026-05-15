let r = try {
        Date.WeekOfYear(#date(2024, 1, 1)),
        Date.WeekOfYear(#date(2024, 6, 15)),
        Date.WeekOfYear(#date(2024, 12, 31)),
        Date.WeekOfYear(#date(2024, 1, 1), Day.Monday)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
