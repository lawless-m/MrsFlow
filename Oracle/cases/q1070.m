// DateTime.ToText standard codes (basic subset).
let dt = #datetime(2026, 6, 15, 10, 30, 45) in
let r = try {
        DateTime.ToText(dt, "yyyy-MM-dd HH:mm:ss"),
        DateTime.ToText(dt, "yyyy"),
        DateTime.ToText(dt, "HH:mm:ss"),
        DateTime.ToText(dt, "dd/MM/yyyy"),
        DateTime.ToText(dt, "yyyy-MM-ddTHH:mm:ss")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
