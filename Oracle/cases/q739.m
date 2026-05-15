// Multi-section "pos;neg;zero" patterns.
let r = try {
        Number.ToText(1.5, "0.00;-0.00;zero"),
        Number.ToText(-1.5, "0.00;-0.00;zero"),
        Number.ToText(0, "0.00;-0.00;zero"),
        Number.ToText(1.5, "0.00;(0.00)"),
        Number.ToText(-1.5, "0.00;(0.00)"),
        Number.ToText(0, "0.00;(0.00)"),
        try Number.ToText(Number.NaN, "0.00") otherwise "err",
        try Number.ToText(Number.PositiveInfinity, "0.00") otherwise "err",
        try Number.ToText(null, "0.00") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
