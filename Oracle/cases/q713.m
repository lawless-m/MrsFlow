// C culture sweep — en-US/GB/JP, de-DE, fr-FR.
let r = try {
        Number.ToText(1234.5, "C2", "en-US"),
        Number.ToText(1234.5, "C2", "en-GB"),
        Number.ToText(1234.5, "C0", "ja-JP"),
        Number.ToText(1234.5, "C2", "de-DE"),
        Number.ToText(1234.5, "C2", "fr-FR"),
        Number.ToText(-1234.5, "C2", "en-US"),
        Number.ToText(-1234.5, "C2", "de-DE"),
        Number.ToText(-1234.5, "C2", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
