// DateTime + Duration with sub-day components.
let r = try {
        #datetime(2026, 6, 15, 10, 30, 0) + #duration(0, 5, 0, 0),
        #datetime(2026, 6, 15, 23, 59, 0) + #duration(0, 0, 2, 0),
        #datetime(2026, 6, 15, 10, 0, 0) - #duration(0, 11, 0, 0),
        #datetime(2026, 6, 15, 10, 30, 0) + #duration(0, 0, 0, 0.5),
        #datetime(2026, 12, 31, 23, 59, 59) + #duration(0, 0, 0, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
