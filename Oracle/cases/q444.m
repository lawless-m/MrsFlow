let r = try {
        Date.EndOfWeek(#date(2024, 6, 15)),
        Date.EndOfWeek(#date(2024, 6, 15), Day.Sunday),
        Date.EndOfWeek(#date(2024, 6, 15), Day.Monday),
        Date.EndOfWeek(#date(2024, 12, 29))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
