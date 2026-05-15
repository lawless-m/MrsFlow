// List.Dates leap-year boundary.
let r = try {
        List.Dates(#date(2024, 2, 28), 3, #duration(1, 0, 0, 0)),
        List.Dates(#date(2025, 2, 28), 3, #duration(1, 0, 0, 0)),
        List.Dates(#date(2024, 12, 30), 5, #duration(1, 0, 0, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
