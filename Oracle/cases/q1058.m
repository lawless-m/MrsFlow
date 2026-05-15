// Record.RemoveFields basic.
let r = try {
        Record.RemoveFields([a=1, b=2, c=3], {"b"}),
        Record.RemoveFields([a=1, b=2], {"a", "b"}),
        Record.RemoveFields([a=1], {}),
        // Missing field — default behaviour.
        Record.RemoveFields([a=1], {"missing"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
