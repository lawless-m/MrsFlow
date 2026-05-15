// Date + Date errors; Date + DateTime; Duration + Time.
let r = try {
        // Date - Date works (returns Duration).
        #date(2026, 6, 15) - #date(2026, 6, 1),
        // DateTime - Duration works.
        #datetime(2026, 6, 15, 10, 0, 0) - #duration(0, 5, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
