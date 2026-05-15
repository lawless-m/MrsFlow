// P thousands-grouping with large fraction inputs.
let r = try {
        Number.ToText(12.345, "P2"),
        Number.ToText(12345.6789, "P0"),
        Number.ToText(12345.6789, "P2"),
        Number.ToText(12345.6789, "P2", "de-DE"),
        Number.ToText(12345.6789, "P2", "fr-FR"),
        Number.ToText(-12345.6789, "P2"),
        Number.ToText(-12345.6789, "P2", "de-DE")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
