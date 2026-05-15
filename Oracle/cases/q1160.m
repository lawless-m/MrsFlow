// DateTimeZone.ToText with short z (hour-only) and zz (two-digit hour).
let dtz_pos = #datetimezone(2026, 6, 15, 10, 30, 0, 5, 0) in
let dtz_utc = #datetimezone(2026, 6, 15, 10, 30, 0, 0, 0) in
let dtz_neg = #datetimezone(2026, 6, 15, 10, 30, 0, -3, 0) in
let r = try {
        DateTimeZone.ToText(dtz_pos, [Format="z"]),
        DateTimeZone.ToText(dtz_utc, [Format="z"]),
        DateTimeZone.ToText(dtz_neg, [Format="z"]),
        DateTimeZone.ToText(dtz_pos, [Format="zz"]),
        DateTimeZone.ToText(dtz_neg, [Format="zz"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
