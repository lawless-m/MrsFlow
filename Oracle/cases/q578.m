let r = try {
        Record.TransformFields([a=1, b=2, c=3], {{"a", each _ * 10}}),
        Record.TransformFields([n=5], {{"n", Text.From}}),
        Record.TransformFields([a=1, b=2], {{"a", each _ + 100}, {"b", each _ - 1}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
