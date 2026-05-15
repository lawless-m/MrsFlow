// DateTime.From + DateTime.FromText edge.
let r = try {
        DateTime.FromText("2026-06-15"),
        DateTime.FromText("2026-06-15T10:30:00"),
        DateTime.FromText("invalid")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
