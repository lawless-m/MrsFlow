let r = try Record.AddField([a=1], "b", () => 99, true) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else Record.FieldNames(r[Value])
