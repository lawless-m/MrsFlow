let r = try Record.Combine({[a=1, b=2], [b=20, c=3], [c=30]}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
