let r = try {
        Time.From(#time(14, 30, 45)),
        Time.From(#datetime(2024, 6, 15, 9, 15, 30)),
        Time.From(0.5),
        Time.From(0.75)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
