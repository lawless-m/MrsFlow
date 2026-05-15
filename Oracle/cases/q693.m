// F20 — very high precision, exposes float representation.
let r = try {
        Number.ToText(0.1, "F20"),
        Number.ToText(0.2, "F20"),
        Number.ToText(0.3, "F20"),
        Number.ToText(1, "F20"),
        Number.ToText(0, "F20"),
        Number.ToText(-0.1, "F20")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
