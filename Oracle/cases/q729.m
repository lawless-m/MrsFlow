// E with very large / very small magnitudes.
let r = try {
        Number.ToText(1e100, "E2"),
        Number.ToText(1e-100, "E2"),
        Number.ToText(1e308, "E2"),
        Number.ToText(1e-308, "E2"),
        Number.ToText(2.225e-308, "E5"),
        Number.ToText(1.7976931348623157e308, "E5"),
        Number.ToText(-1e100, "E3"),
        Number.ToText(-1e-100, "E3")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
