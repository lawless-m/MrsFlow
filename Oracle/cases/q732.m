// E exponent zero-padding (always 3 digits in .NET, with sign).
let r = try {
        Number.ToText(1, "E2"),
        Number.ToText(10, "E2"),
        Number.ToText(100, "E2"),
        Number.ToText(1000, "E2"),
        Number.ToText(1e10, "E2"),
        Number.ToText(0.1, "E2"),
        Number.ToText(0.01, "E2"),
        Number.ToText(0.001, "E2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
