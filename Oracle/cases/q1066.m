// Date subtraction — Date - Date = Duration.
let r = try {
        #date(2026, 6, 15) - #date(2026, 6, 10),
        #date(2026, 6, 10) - #date(2026, 6, 15),
        #date(2026, 3, 1) - #date(2026, 2, 28),
        #date(2024, 3, 1) - #date(2024, 2, 28)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
