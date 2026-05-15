// Text.Combine with non-text element — does PQ coerce or refuse?
let r = try {
        Text.Combine({"a", 42, "b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
