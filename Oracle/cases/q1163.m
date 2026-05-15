// DateTimeZone.ToText round-trip via DateTimeZone.FromText.
let original = #datetimezone(2026, 6, 15, 10, 30, 45, 2, 0) in
let r = try {
        DateTimeZone.ToText(original, [Format="o"]),
        DateTimeZone.FromText(DateTimeZone.ToText(original, [Format="o"])) = original,
        DateTimeZone.FromText("2026-06-15T10:30:00+02:00") = #datetimezone(2026, 6, 15, 10, 30, 0, 2, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
