// Text.Combine separator types.
let r = try {
        Text.Combine({"a", "b"}, ""),
        Text.Combine({"a", "b"}, " - "),
        Text.Combine({"a", "b"}, "→"),
        Text.Combine({"a", "b"}, null),
        Text.Combine({"a", "b"}, "#(cr,lf)")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
