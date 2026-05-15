// DateTimeZone.ToText standard format specifiers.
let dtz = #datetimezone(2026, 6, 15, 10, 30, 45, 1, 0) in
let r = try {
        DateTimeZone.ToText(dtz, [Format="o"]),
        DateTimeZone.ToText(dtz, [Format="s"]),
        DateTimeZone.ToText(dtz, [Format="u"]),
        DateTimeZone.ToText(dtz, [Format="R"]),
        DateTimeZone.ToText(dtz, [Format="yyyy-MM-dd HH:mm:ss"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
