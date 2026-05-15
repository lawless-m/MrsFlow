// Very small and very large magnitude.
let r = try {
        Number.ToText(1e-10, "P10"),
        Number.ToText(1e-15, "P15"),
        Number.ToText(100, "P0"),
        Number.ToText(1000, "P0"),
        Number.ToText(0.000001, "P4"),
        Number.ToText(0.000001, "P8"),
        Number.ToText(1e10, "P0"),
        Number.ToText(-1e6, "P2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
