// F precision combined with culture (decimal-separator).
let r = try {
        Number.ToText(3.14, "F2", "en-US"),
        Number.ToText(3.14, "F2", "en-GB"),
        Number.ToText(3.14, "F2", "de-DE"),
        Number.ToText(3.14, "F2", "fr-FR"),
        Number.ToText(1234.5, "F2", "en-US"),
        Number.ToText(1234.5, "F2", "de-DE"),
        Number.ToText(-0.5, "F1", "de-DE")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
