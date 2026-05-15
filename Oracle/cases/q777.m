// Text.PadStart unicode pad-char + counting (PQ counts characters not bytes).
let r = try {
        Text.PadStart("a", 5, "→"),
        Text.PadStart("café", 6, "*"),
        Text.PadStart("→", 3, "X"),
        Text.PadStart("ß", 4, "→")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
