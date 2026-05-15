// N rounding behaviour (same away-from-zero as F).
let r = try {
        Number.ToText(0.5, "N0"),
        Number.ToText(1.5, "N0"),
        Number.ToText(2.5, "N0"),
        Number.ToText(-0.5, "N0"),
        Number.ToText(-1.5, "N0"),
        Number.ToText(0.005, "N2"),
        Number.ToText(0.015, "N2"),
        Number.ToText(0.025, "N2"),
        Number.ToText(0.035, "N2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
