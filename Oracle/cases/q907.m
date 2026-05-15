// List.Dates with negative step (backwards).
let r = try {
        List.Dates(#date(2026, 1, 5), 5, #duration(-1, 0, 0, 0)),
        List.Dates(#date(2026, 3, 1), 3, #duration(-30, 0, 0, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
