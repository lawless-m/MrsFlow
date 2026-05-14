let r = try
    let v = Record.AddField([a=1], "bad", () => error "x", true)[bad] in
        if Value.Is(v, type function) then v() else v
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
