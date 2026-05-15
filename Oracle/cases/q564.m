let r = try {
        Text.Trim("abcxyz", {"a", "z"}),
        Text.TrimStart("abcabc", {"a", "b"}),
        Text.TrimEnd("xxyyzz", {"y", "z"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
