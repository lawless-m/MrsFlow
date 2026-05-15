// Text.Contains with culture-sensitive chars: default Ordinal (codepoint).
let r = try {
        Text.Contains("café", "café"),
        Text.Contains("Straße", "ass"),
        Text.Contains("İSTANBUL", "ist"),
        Text.Contains("naïve", "ai"),
        Text.Contains("Æther", "ae"),
        Text.Contains("", ""),
        Text.Contains("hello", "")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
