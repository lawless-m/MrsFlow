// E precision sweep.
let r = try {
        Number.ToText(3.14159265358979, "E0"),
        Number.ToText(3.14159265358979, "E1"),
        Number.ToText(3.14159265358979, "E5"),
        Number.ToText(3.14159265358979, "E10"),
        Number.ToText(3.14159265358979, "E15"),
        Number.ToText(3.14159265358979, "E20"),
        Number.ToText(1, "E5"),
        Number.ToText(0.5, "E5")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
