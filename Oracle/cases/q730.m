// E with NaN/Inf/null.
let r = try {
        try Number.ToText(Number.NaN, "E2") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "E2") otherwise "err",
        try Number.ToText(Number.NegativeInfinity, "E2") otherwise "err",
        try Number.ToText(null, "E2") otherwise "err",
        Number.ToText(0.0, "E2"),
        Number.ToText(-0.0, "E2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
