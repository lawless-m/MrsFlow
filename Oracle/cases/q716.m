// C edge values.
let r = try {
        try Number.ToText(Number.NaN, "C2") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "C2") otherwise "err",
        try Number.ToText(Number.NegativeInfinity, "C2") otherwise "err",
        try Number.ToText(null, "C2") otherwise "err",
        Number.ToText(1234.5, "C"),
        Number.ToText(1234.5, "c0"),
        Number.ToText(1234.5, "c2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
