// D with NaN/Inf/null.
let r = try {
        try Number.ToText(Number.NaN, "D5") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "D5") otherwise "err",
        try Number.ToText(Number.NegativeInfinity, "D5") otherwise "err",
        try Number.ToText(null, "D5") otherwise "err",
        Number.ToText(42, "d"),
        Number.ToText(42, "d3")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
