// N0 / N2 — basic thousand grouping with no precision and 2-digit.
let r = try {
        Number.ToText(1234567, "N0"),
        Number.ToText(1234567, "N2"),
        Number.ToText(1234567.89, "N0"),
        Number.ToText(1234567.89, "N2"),
        Number.ToText(0, "N0"),
        Number.ToText(0, "N2"),
        Number.ToText(-1234567.89, "N2"),
        Number.ToText(999, "N2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
