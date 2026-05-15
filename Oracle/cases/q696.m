// F with NaN/Inf/null + invalid F999 precision.
let r = try {
        try Number.ToText(Number.NaN, "F2") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "F2") otherwise "err",
        try Number.ToText(Number.NegativeInfinity, "F2") otherwise "err",
        try Number.ToText(null, "F2") otherwise "err",
        Number.ToText(3.14159, "F"),
        Number.ToText(3.14159, "f0"),
        Number.ToText(3.14159, "f2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
