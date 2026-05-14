let r = try Time.ToText(#time(14,30,0), "hh:mm tt") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
