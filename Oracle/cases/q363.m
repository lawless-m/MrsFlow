let r = try {Text.Repeat("ab", 3), Text.Repeat("x", 0), Text.Repeat("", 5), Text.Repeat("-", 10)} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
