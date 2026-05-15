// N with NaN/Inf/null.
let r = try {
        try Number.ToText(Number.NaN, "N2") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "N2") otherwise "err",
        try Number.ToText(Number.NegativeInfinity, "N2") otherwise "err",
        try Number.ToText(null, "N2") otherwise "err",
        Number.ToText(1234.5, "N"),
        Number.ToText(1234.5, "n2"),
        Number.ToText(1234.5, "n0")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
