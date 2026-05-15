let r = try {
        Date.AddDays(#date(2024, 6, 15), 10),
        #date(2024, 6, 15) + #duration(7, 0, 0, 0),
        #date(2024, 6, 15) - #duration(1, 0, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
