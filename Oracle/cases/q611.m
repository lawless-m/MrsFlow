let r = try {
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyy-MM-dd HH:mm:ss"),
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyy/MM/dd"),
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "dd-MMM-yyyy"),
        DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "HH:mm")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
