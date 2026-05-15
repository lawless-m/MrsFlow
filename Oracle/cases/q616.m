let r = try {
        Date.AddDays(#date(2024, 12, 31), 1),
        Date.AddDays(#date(2024, 12, 31), 365),
        Date.AddDays(#date(2025, 1, 1), -1),
        Date.AddDays(#date(2024, 1, 1), -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
