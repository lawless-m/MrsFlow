// Variant nibble — RFC 4122 v4 requires the first char of segment 4 ∈ {8,9,a,b}.
let g = Text.NewGuid() in
let variantChar = Text.Range(g, 19, 1) in
let r = try {
        List.Contains({"8", "9", "a", "b"}, variantChar),
        // Ensure result is lowercase (no uppercase hex).
        Text.Upper(g) <> g,
        Text.Lower(g) = g
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
