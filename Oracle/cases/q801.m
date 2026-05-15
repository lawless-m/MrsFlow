// Text.TrimStart vs TrimEnd vs Trim on the same input.
let r = try {
        Text.TrimStart("XXabcXX", "X"),
        Text.TrimEnd("XXabcXX", "X"),
        Text.Trim("XXabcXX", "X"),
        Text.TrimStart("  abc  "),
        Text.TrimEnd("  abc  ")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
