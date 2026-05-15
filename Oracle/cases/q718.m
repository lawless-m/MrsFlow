// Negative-currency formatting — .NET shows parentheses in en-US.
let r = try {
        Number.ToText(-100, "C2", "en-US"),
        Number.ToText(-100, "C2", "en-GB"),
        Number.ToText(-100, "C0", "ja-JP"),
        Number.ToText(-100, "C2", "de-DE"),
        Number.ToText(-100, "C2", "fr-FR"),
        Number.ToText(0, "C2", "en-US"),
        Number.ToText(0, "C2", "de-DE"),
        Number.ToText(0, "C2", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
