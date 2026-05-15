// P culture sweep.
let r = try {
        Number.ToText(0.5, "P2", "en-US"),
        Number.ToText(0.5, "P2", "en-GB"),
        Number.ToText(0.5, "P2", "de-DE"),
        Number.ToText(0.5, "P2", "fr-FR"),
        Number.ToText(0.123, "P2", "en-US"),
        Number.ToText(0.123, "P2", "de-DE"),
        Number.ToText(-0.123, "P2", "fr-FR"),
        Number.ToText(1234.5, "P2", "en-US")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
