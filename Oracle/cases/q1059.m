// Record.HasFields / Record.Field.
let r = try {
        Record.HasFields([a=1, b=2], "a"),
        Record.HasFields([a=1, b=2], "missing"),
        Record.HasFields([a=1, b=2], {"a", "b"}),
        Record.HasFields([a=1, b=2], {"a", "missing"}),
        Record.Field([a=1, b=2], "a"),
        Record.Field([a=1, b=2], "missing")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
