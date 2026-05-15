// Rounding behaviour at half-points (away-from-zero).
let r = try {
        Number.ToText(0.005, "P0"),
        Number.ToText(0.015, "P0"),
        Number.ToText(0.025, "P0"),
        Number.ToText(0.005, "P1"),
        Number.ToText(0.015, "P1"),
        Number.ToText(0.025, "P1"),
        Number.ToText(-0.005, "P0"),
        Number.ToText(-0.015, "P0")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
