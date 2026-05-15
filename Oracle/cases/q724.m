// D with culture (.NET's D doesn't use culture for digits, only for digits themselves).
// Cultures shouldn't affect plain digit output.
let r = try {
        Number.ToText(42, "D5", "en-US"),
        Number.ToText(42, "D5", "en-GB"),
        Number.ToText(42, "D5", "de-DE"),
        Number.ToText(42, "D5", "fr-FR"),
        Number.ToText(42, "D5", "ja-JP"),
        Number.ToText(-42, "D5", "de-DE"),
        Number.ToText(0, "D5", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
