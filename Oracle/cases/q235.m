let r = try Record.AddField([a=1], "bad", () => error "x", true)[bad] in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
