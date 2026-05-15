// # vs 0 — # drops, 0 zero-pads.
let r = try {
        Number.ToText(3.14, "#.##"),
        Number.ToText(3.1, "#.##"),
        Number.ToText(3, "#.##"),
        Number.ToText(3.14, "#.00"),
        Number.ToText(3.1, "#.00"),
        Number.ToText(3, "#.00"),
        Number.ToText(0.5, "#.##"),
        Number.ToText(0.5, "0.##")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
