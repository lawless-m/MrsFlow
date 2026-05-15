// Text.Combine with empty strings.
let r = try {
        Text.Combine({"", ""}, ","),
        Text.Combine({"a", ""}, ","),
        Text.Combine({"", "a"}, ","),
        Text.Combine({"", "", ""}, "-"),
        Text.Combine({""}, ",")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
