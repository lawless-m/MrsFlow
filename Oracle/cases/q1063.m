// Date.AddYears(-100), -1000.
let r = try {
        Date.AddYears(#date(2026, 6, 15), -100),
        Date.AddYears(#date(2026, 6, 15), -1000),
        Date.AddYears(#date(1, 1, 1), 0),
        Date.AddYears(#date(9999, 12, 31), 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
