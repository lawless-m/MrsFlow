// Date + Date should error (no semantics for sum of two dates).
let r = try {
        #date(2026, 6, 15) + #date(2026, 6, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
