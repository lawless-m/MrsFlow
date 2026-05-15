// Basic 0.00 / 0.000 patterns.
let r = try {
        Number.ToText(3.14159, "0.00"),
        Number.ToText(3.14159, "0.000"),
        Number.ToText(3.14159, "0.00000"),
        Number.ToText(0, "0.00"),
        Number.ToText(0.5, "0.00"),
        Number.ToText(-3.14159, "0.00"),
        Number.ToText(1234, "0.00"),
        Number.ToText(1234.5, "0")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
