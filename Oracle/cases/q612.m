let r = try {
        DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "yyyy-MM-dd"),
        DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "yyyy-M-d"),
        DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "H:m:s"),
        DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "HH:mm:ss")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
