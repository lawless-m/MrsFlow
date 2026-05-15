let r = try {
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "dddd"),
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "ddd"),
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "MMMM"),
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "MMM")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
