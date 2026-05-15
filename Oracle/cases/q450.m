let r = try {
        Date.FromText("June 15, 2024", "en-US"),
        Date.FromText("15 Juni 2024", "de-DE")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
