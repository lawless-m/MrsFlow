// C precision sweep + large magnitudes.
let r = try {
        Number.ToText(1234567.89, "C0", "en-US"),
        Number.ToText(1234567.89, "C2", "en-US"),
        Number.ToText(1234567.89, "C4", "en-US"),
        Number.ToText(1234567.89, "C0", "en-GB"),
        Number.ToText(1234567.89, "C2", "en-GB"),
        Number.ToText(1234567.89, "C0", "ja-JP"),
        Number.ToText(1234567.89, "C2", "de-DE"),
        Number.ToText(1234567.89, "C2", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
