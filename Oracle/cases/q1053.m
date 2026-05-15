// Record.TransformFields basic — apply function to a single named field.
let r = try {
        Record.TransformFields([a=1, b=2, c=3], {"a", each _ * 10}),
        Record.TransformFields([a=1, b=2], {"a", each Text.From(_)}),
        Record.TransformFields([a=1], {"a", each _})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
