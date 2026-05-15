let r = try List.Generate(() => 0, each _ < 5, each _ + 1) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
