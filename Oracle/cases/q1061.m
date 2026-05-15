// Date.AddMonths at end-of-month — Jan 31 + 1 month = Feb 28/29?
let r = try {
        Date.AddMonths(#date(2026, 1, 31), 1),
        Date.AddMonths(#date(2024, 1, 31), 1),
        Date.AddMonths(#date(2026, 3, 31), -1),
        Date.AddMonths(#date(2026, 12, 31), 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
