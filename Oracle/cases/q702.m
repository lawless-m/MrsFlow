// N very large + very small.
let r = try {
        Number.ToText(1e15, "N0"),
        Number.ToText(1e15, "N2"),
        Number.ToText(1e-5, "N6"),
        Number.ToText(1e-10, "N12"),
        Number.ToText(123456789012345, "N0"),
        Number.ToText(-123456789012345, "N0"),
        Number.ToText(1e10, "N0", "de-DE"),
        Number.ToText(1e10, "N0", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
