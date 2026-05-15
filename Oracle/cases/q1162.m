// DateTimeZone.ToText default (no Format) emits ISO-8601 with tz.
let r = try {
        DateTimeZone.ToText(#datetimezone(2026, 6, 15, 10, 30, 0, 1, 0)),
        DateTimeZone.ToText(#datetimezone(2026, 6, 15, 10, 30, 0, 0, 0)),
        DateTimeZone.ToText(#datetimezone(2026, 6, 15, 10, 30, 0, -5, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
