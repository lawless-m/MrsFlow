// D basic width sweep.
let r = try {
        Number.ToText(42, "D"),
        Number.ToText(42, "D0"),
        Number.ToText(42, "D1"),
        Number.ToText(42, "D5"),
        Number.ToText(42, "D10"),
        Number.ToText(42, "D20"),
        Number.ToText(0, "D5"),
        Number.ToText(7, "D3")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
