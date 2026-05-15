let r = try {
        Date.StartOfWeek(#date(2024, 6, 15)),
        Date.StartOfWeek(#date(2024, 6, 15), Day.Sunday),
        Date.StartOfWeek(#date(2024, 6, 15), Day.Monday),
        Date.StartOfWeek(#date(2024, 6, 17), Day.Monday)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
