let r = try Record.Field(Record.Combine({[a=1], [a=2]}), "a") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
