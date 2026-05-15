// Record.AddField with delayed=true — flag is silently accepted but a
// proper implementation would defer the value-expression until access.
// Probe that adding a plain (non-function) value with delayed=true
// still stores the value, and that AddField allows a 4-arg form
// without erroring on the flag itself.
let r = try {
        Record.AddField([a=1], "b", 99, true),
        Record.AddField([a=1], "b", 99, false),
        Record.AddField([a=1], "b", 99, true)[b]
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
