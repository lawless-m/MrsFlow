let r = try DateTimeZone.ToText(
    #datetimezone(2026,6,15,14,30,45,1,0), "K") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
