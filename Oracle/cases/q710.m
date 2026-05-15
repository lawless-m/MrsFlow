// P with NaN/Inf/null + case-insensitive p.
let r = try {
        try Number.ToText(Number.NaN, "P2") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "P2") otherwise "err",
        try Number.ToText(Number.NegativeInfinity, "P2") otherwise "err",
        try Number.ToText(null, "P2") otherwise "err",
        Number.ToText(0.5, "P"),
        Number.ToText(0.5, "p0"),
        Number.ToText(0.5, "p2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
