let r = try {
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "tt"),
        DateTime.ToText(#datetime(2024, 6, 15, 9, 30, 0), "h:mm tt"),
        DateTime.ToText(#datetime(2024, 6, 15, 23, 59, 0), "h:mm tt"),
        DateTime.ToText(#datetime(2024, 6, 15, 0, 0, 0), "h:mm tt")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
