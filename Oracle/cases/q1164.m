// DateTimeZone.ToText edge cases: half-hour offsets, fractional second.
let dtz_half = #datetimezone(2026, 6, 15, 10, 30, 0, 5, 30) in
let dtz_45 = #datetimezone(2026, 6, 15, 10, 30, 0, 5, 45) in
let r = try {
        DateTimeZone.ToText(dtz_half, [Format="K"]),
        DateTimeZone.ToText(dtz_45, [Format="K"]),
        DateTimeZone.ToText(dtz_half, [Format="zzz"]),
        DateTimeZone.ToText(dtz_45, [Format="zzz"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
