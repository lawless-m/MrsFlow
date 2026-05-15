// E rounding behaviour at half-points in mantissa.
let r = try {
        Number.ToText(1.5, "E0"),
        Number.ToText(2.5, "E0"),
        Number.ToText(3.5, "E0"),
        Number.ToText(-1.5, "E0"),
        Number.ToText(-2.5, "E0"),
        Number.ToText(1.25, "E1"),
        Number.ToText(2.25, "E1"),
        Number.ToText(0.0005, "E0"),
        Number.ToText(0.0015, "E0")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
