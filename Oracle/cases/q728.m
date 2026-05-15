// E case (E uppercase vs e lowercase) + culture.
let r = try {
        Number.ToText(1234.5, "E2"),
        Number.ToText(1234.5, "e2"),
        Number.ToText(1234.5, "E2", "en-US"),
        Number.ToText(1234.5, "E2", "de-DE"),
        Number.ToText(1234.5, "E2", "fr-FR"),
        Number.ToText(-1234.5, "e2", "de-DE"),
        Number.ToText(0.0001, "e3", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
