// Fraction inputs hitting awkward rounding.
let r = try {
        Number.ToText(0.125, "P0"),
        Number.ToText(0.125, "P1"),
        Number.ToText(0.125, "P2"),
        Number.ToText(0.001, "P0"),
        Number.ToText(0.001, "P2"),
        Number.ToText(0.0001, "P2"),
        Number.ToText(0.0001, "P4"),
        Number.ToText(0.9999, "P0"),
        Number.ToText(0.9999, "P2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
