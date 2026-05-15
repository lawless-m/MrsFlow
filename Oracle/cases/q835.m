// Text.Combine basic — empty list, single, multi.
let r = try {
        Text.Combine({}),
        Text.Combine({"a"}),
        Text.Combine({"a", "b"}),
        Text.Combine({"a", "b", "c"}),
        Text.Combine({"a", "b"}, ", "),
        Text.Combine({"a"}, ", "),
        Text.Combine({}, ", ")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
