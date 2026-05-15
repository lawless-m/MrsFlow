let r = try {
        Date.FromText("2024-06-15"),
        Date.FromText("2024-12-31"),
        Date.FromText("2024-02-29")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
