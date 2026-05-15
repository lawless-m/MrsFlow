let r = try Record.FieldNames(Record.AddField([a=1], "b", () => 42, true)) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
