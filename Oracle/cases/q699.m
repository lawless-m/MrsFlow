// N higher/lower precisions.
let r = try {
        Number.ToText(1234567.123456, "N1"),
        Number.ToText(1234567.123456, "N3"),
        Number.ToText(1234567.123456, "N5"),
        Number.ToText(1234567.123456, "N10"),
        Number.ToText(0.5, "N2"),
        Number.ToText(0.05, "N2"),
        Number.ToText(0.005, "N2"),
        Number.ToText(0.0005, "N2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
