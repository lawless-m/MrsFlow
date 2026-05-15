// F0 / F1 / F2 — common precisions.
let r = try {
        Number.ToText(3.14159, "F0"),
        Number.ToText(3.14159, "F1"),
        Number.ToText(3.14159, "F2"),
        Number.ToText(0, "F2"),
        Number.ToText(-3.14159, "F2"),
        Number.ToText(0.5, "F0"),
        Number.ToText(1.5, "F0"),
        Number.ToText(2.5, "F0")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
