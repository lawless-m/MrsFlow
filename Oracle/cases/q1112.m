// DateTimeZone + Duration preserves zone.
let r = try {
        #datetimezone(2026, 6, 15, 10, 30, 0, 1, 0) + #duration(0, 5, 0, 0),
        #datetimezone(2026, 6, 15, 10, 0, 0, -5, 0) - #duration(0, 0, 30, 0),
        #datetimezone(2026, 6, 15, 23, 30, 0, 0, 0) + #duration(0, 1, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
