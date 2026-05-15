// D padding boundary cases.
let r = try {
        Number.ToText(99999, "D5"),
        Number.ToText(100000, "D5"),
        Number.ToText(100000, "D6"),
        Number.ToText(123456789, "D5"),
        Number.ToText(123456789, "D20"),
        Number.ToText(-99999, "D5"),
        Number.ToText(-100000, "D5"),
        Number.ToText(-100000, "D6")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
