let r = try Record.FieldNames(Record.AddField([a=1], "b", 2, false)) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
