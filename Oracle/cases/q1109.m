// Date + Duration / Date - Duration: day-precision rollover.
let r = try {
        #date(2026, 6, 15) + #duration(7, 0, 0, 0),
        #date(2026, 6, 15) - #duration(20, 0, 0, 0),
        #date(2026, 12, 31) + #duration(1, 0, 0, 0),
        #date(2026, 3, 1) - #duration(1, 0, 0, 0),
        #date(2024, 2, 29) + #duration(365, 0, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
