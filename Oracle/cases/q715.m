// C rounding half-points.
let r = try {
        Number.ToText(0.5, "C0"),
        Number.ToText(1.5, "C0"),
        Number.ToText(2.5, "C0"),
        Number.ToText(-0.5, "C0"),
        Number.ToText(-1.5, "C0"),
        Number.ToText(0.005, "C2"),
        Number.ToText(0.015, "C2"),
        Number.ToText(0.025, "C2"),
        Number.ToText(-0.005, "C2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
