// Mixed custom formats with date + time + zone components.
let dtz = #datetimezone(2026, 6, 15, 10, 30, 45, 1, 30) in
let r = try {
        DateTimeZone.ToText(dtz, [Format="yyyy-MM-ddTHH:mm:ssK"]),
        DateTimeZone.ToText(dtz, [Format="dd MMM yyyy HH:mm zzz", Culture="en-US"]),
        DateTimeZone.ToText(dtz, [Format="HH:mm K"]),
        DateTimeZone.ToText(dtz, [Format="yyyy/MM/dd"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
