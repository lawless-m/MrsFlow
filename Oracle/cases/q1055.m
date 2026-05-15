// Record.TransformFields with missing field name.
let r = try {
        Record.TransformFields([a=1], {"missing", each _ * 10}),
        // Missing field with MissingField.Ignore.
        Record.TransformFields([a=1], {"missing", each _ * 10}, MissingField.Ignore)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
