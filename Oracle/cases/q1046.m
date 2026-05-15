// Record.AddField basic — 3-arg form (no delayed flag).
let r = try {
        Record.AddField([a=1, b=2], "c", 3),
        Record.AddField([], "x", 100),
        Record.AddField([a=1], "b", null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
