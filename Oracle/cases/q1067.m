// DateTime.From with various input types.
let r = try {
        DateTime.From("2026-06-15T10:30:00"),
        DateTime.From(#date(2026, 6, 15)),
        DateTime.From(#datetime(2026, 6, 15, 10, 30, 0)),
        DateTime.From(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
