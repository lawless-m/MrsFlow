// Date.WeekOfYear with various Day.* values across year boundary.
let r = try {
        Date.WeekOfYear(#date(2026, 1, 1)),
        Date.WeekOfYear(#date(2026, 1, 1), Day.Monday),
        Date.WeekOfYear(#date(2026, 1, 1), Day.Sunday),
        Date.WeekOfYear(#date(2026, 12, 31), Day.Monday),
        Date.WeekOfYear(#date(2025, 12, 31), Day.Sunday)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
