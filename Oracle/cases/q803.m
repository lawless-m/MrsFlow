// Text.Trim edge cases — null inputs, empty trim set, all-trim text.
let r = try {
        Text.Trim(null),
        Text.Trim("abc", null),
        Text.Trim("XXXX", "X"),
        Text.Trim("XXXX", {})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
