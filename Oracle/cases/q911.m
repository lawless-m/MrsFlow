// List.Dates with zero-step → infinite same date repeated count times.
let r = try {
        List.Dates(#date(2026, 1, 1), 3, #duration(0, 0, 0, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
