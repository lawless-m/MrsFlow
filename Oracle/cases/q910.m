// List.Dates count = 0 / negative.
let r = try {
        List.Dates(#date(2026, 1, 1), 0, #duration(1, 0, 0, 0)),
        List.Dates(#date(2026, 1, 1), -1, #duration(1, 0, 0, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
