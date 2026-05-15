// E default precision (6) + sign cases.
let r = try {
        Number.ToText(1234.5, "E"),
        Number.ToText(1234.5, "E0"),
        Number.ToText(1234.5, "E2"),
        Number.ToText(1234.5, "E6"),
        Number.ToText(0, "E2"),
        Number.ToText(-1234.5, "E2"),
        Number.ToText(0.000123, "E2"),
        Number.ToText(-0.000123, "E2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
