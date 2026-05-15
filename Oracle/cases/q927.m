// List.Transform 1-arg form.
let r = try {
        List.Transform({1, 2, 3}, each _ * 2),
        List.Transform({}, each _ * 2),
        List.Transform({"a", "b"}, each Text.Upper(_))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
