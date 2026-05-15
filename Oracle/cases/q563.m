let r = try {
        Text.TrimStart("  hello  "),
        Text.TrimEnd("  hello  "),
        Text.TrimStart("xxhelloxx", "x"),
        Text.TrimEnd("xxhelloxx", "x")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
