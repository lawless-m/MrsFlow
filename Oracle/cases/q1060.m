// Date.AddYears at leap-day boundary — Feb 29 → following non-leap year.
let r = try {
        Date.AddYears(#date(2024, 2, 29), 1),
        Date.AddYears(#date(2024, 2, 29), 4),
        Date.AddYears(#date(2024, 2, 28), 1),
        Date.AddYears(#date(2000, 2, 29), -100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
