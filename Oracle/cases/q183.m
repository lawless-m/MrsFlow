let r = try DateTime.ToText(#datetime(2026,6,15,14,30,45), "f") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
