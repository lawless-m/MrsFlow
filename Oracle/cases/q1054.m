// Record.TransformFields list-of-pairs form.
let r = try {
        Record.TransformFields([a=1, b=2, c=3], {{"a", each _ * 10}, {"c", each _ + 100}}),
        Record.TransformFields([a="hello"], {{"a", each Text.Upper(_)}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
