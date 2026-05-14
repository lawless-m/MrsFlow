let r = try Record.AddField([a=1], "bad", () => error "x", true) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else Record.FieldNames(r[Value])
