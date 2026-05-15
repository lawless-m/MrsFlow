// Text.Replace with unicode needles/replacements.
let r = try {
        Text.Replace("café", "é", "e"),
        Text.Replace("naïve", "ï", "i"),
        Text.Replace("→→→", "→", "->"),
        Text.Replace("ßß", "ß", "ss")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
