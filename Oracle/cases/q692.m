// F5 / F10 — high precision.
let r = try {
        Number.ToText(3.14159265358979, "F5"),
        Number.ToText(3.14159265358979, "F10"),
        Number.ToText(0, "F5"),
        Number.ToText(-3.14159265358979, "F5"),
        Number.ToText(1, "F5"),
        Number.ToText(1.23456789, "F5"),
        Number.ToText(1.23456789, "F10")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
