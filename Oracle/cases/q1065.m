// Date.From with various sources.
let r = try {
        Date.From("2026-06-15"),
        Date.From(#datetime(2026, 6, 15, 10, 30, 0)),
        Date.From(45000),
        Date.From(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
