// Date + Duration / DateTime + Duration.
let r = try {
        #date(2026, 6, 15) + #duration(1, 0, 0, 0),
        #date(2026, 6, 15) - #duration(1, 0, 0, 0),
        #datetime(2026, 6, 15, 10, 0, 0) + #duration(0, 5, 30, 0),
        #datetime(2026, 6, 15, 23, 30, 0) + #duration(0, 1, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
