// Scientific in custom pattern.
let r = try {
        Number.ToText(1234.5, "#.##E+0"),
        Number.ToText(1234.5, "0.00E+00"),
        Number.ToText(1234.5, "0.00E+000"),
        Number.ToText(0.001234, "0.00E+00"),
        Number.ToText(-1234.5, "0.00E+00"),
        Number.ToText(0, "0.00E+00"),
        Number.ToText(1, "0.00E+00")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
