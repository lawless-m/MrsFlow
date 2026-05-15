// DateTimeZone.ToText with K (offset, +01:00 form) and zzz (long offset).
let dtz_pos = #datetimezone(2026, 6, 15, 10, 30, 0, 5, 30) in
let dtz_utc = #datetimezone(2026, 6, 15, 10, 30, 0, 0, 0) in
let dtz_neg = #datetimezone(2026, 6, 15, 10, 30, 0, -8, 0) in
let r = try {
        DateTimeZone.ToText(dtz_pos, [Format="K"]),
        DateTimeZone.ToText(dtz_utc, [Format="K"]),
        DateTimeZone.ToText(dtz_neg, [Format="K"]),
        DateTimeZone.ToText(dtz_pos, [Format="zzz"]),
        DateTimeZone.ToText(dtz_utc, [Format="zzz"]),
        DateTimeZone.ToText(dtz_neg, [Format="zzz"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
