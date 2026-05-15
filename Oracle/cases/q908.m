// List.DateTimes — step in hours / minutes / seconds.
let r = try {
        List.DateTimes(#datetime(2026, 1, 1, 0, 0, 0), 4, #duration(0, 6, 0, 0)),
        List.DateTimes(#datetime(2026, 1, 1, 0, 0, 0), 3, #duration(0, 0, 30, 0)),
        List.DateTimes(#datetime(2026, 1, 1, 23, 30, 0), 3, #duration(0, 0, 30, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
