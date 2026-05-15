// Text.Combine with null elements — does PQ treat null as ""?
let r = try {
        Text.Combine({"a", null, "b"}),
        Text.Combine({null, null}),
        Text.Combine({null}),
        Text.Combine({"a", null, "b"}, ","),
        Text.Combine({null, "a", null}, "-")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
