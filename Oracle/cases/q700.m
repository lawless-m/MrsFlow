// N culture: en-US uses "1,234.56"; de-DE uses "1.234,56";
// fr-FR uses NBSP-thousands "1 234,56"; en-GB matches en-US.
let r = try {
        Number.ToText(1234567.89, "N2", "en-US"),
        Number.ToText(1234567.89, "N2", "en-GB"),
        Number.ToText(1234567.89, "N2", "de-DE"),
        Number.ToText(1234567.89, "N2", "fr-FR"),
        Number.ToText(-1234567.89, "N2", "de-DE"),
        Number.ToText(0, "N2", "de-DE"),
        Number.ToText(0.5, "N2", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
