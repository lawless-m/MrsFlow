// Type-mismatch arithmetic that should error: Duration + Date wrong order? Date * 2?
// PQ commutes Date+Duration but Time+Date errors.
let r = try {
        #duration(1, 0, 0, 0) + #date(2026, 6, 15),
        #duration(0, 5, 0, 0) + #datetime(2026, 6, 15, 10, 0, 0),
        #time(10, 0, 0) + #date(2026, 6, 15)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
