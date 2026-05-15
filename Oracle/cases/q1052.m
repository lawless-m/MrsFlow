// Record.FieldOrDefault — present field, missing field with default, null default.
let r = try {
        Record.FieldOrDefault([a=1], "a"),
        Record.FieldOrDefault([a=1], "missing"),
        Record.FieldOrDefault([a=1], "missing", 99),
        Record.FieldOrDefault([a=1], "a", 99),
        Record.FieldOrDefault([a=null], "a"),
        Record.FieldOrDefault([a=null], "a", "fallback")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
