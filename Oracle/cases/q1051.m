// Record.SelectFields with MissingField.* options.
let r = try {
        Record.SelectFields([a=1, b=2, c=3], {"a", "c"}),
        Record.SelectFields([a=1], {"a", "missing"}, MissingField.UseNull),
        Record.SelectFields([a=1], {"a", "missing"}, MissingField.Ignore),
        Record.SelectFields([a=1], {"a", "missing"}, MissingField.Error)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
