// List.Transform 2-arg form (item, index).
let r = try {
        List.Transform({"a", "b", "c"}, (item, idx) => Text.From(idx) & "=" & item),
        List.Transform({10, 20, 30}, (v, i) => v + i),
        List.Transform({}, (v, i) => v + i)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
