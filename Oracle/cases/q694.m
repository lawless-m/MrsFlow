// Rounding behaviour at F0/F1/F2 — banker's vs away.
let r = try {
        Number.ToText(0.5, "F0"),
        Number.ToText(1.5, "F0"),
        Number.ToText(2.5, "F0"),
        Number.ToText(3.5, "F0"),
        Number.ToText(-0.5, "F0"),
        Number.ToText(-1.5, "F0"),
        Number.ToText(0.05, "F1"),
        Number.ToText(0.15, "F1"),
        Number.ToText(0.25, "F1"),
        Number.ToText(0.35, "F1")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
