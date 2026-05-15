// DateTimeZone.ToUtc / ToLocal / SwitchZone basic.
let dtz = #datetimezone(2026, 6, 15, 10, 30, 0, 0, 0) in
let r = try {
        DateTimeZone.ToUtc(dtz),
        DateTimeZone.SwitchZone(dtz, 5, 0),
        DateTimeZone.SwitchZone(dtz, -8, 0),
        DateTimeZone.RemoveZone(dtz)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
