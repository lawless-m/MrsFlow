let r = try {
        Date.IsLeapYear(#date(2024, 1, 1)),
        Date.IsLeapYear(#date(2023, 1, 1)),
        Date.IsLeapYear(#date(2000, 1, 1)),
        Date.IsLeapYear(#date(1900, 1, 1)),
        Date.IsLeapYear(#date(2100, 1, 1))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
