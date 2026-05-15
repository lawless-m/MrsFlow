// P precision sweep. PQ multiplies by 100 and appends "%" (note: PQ
// uses NBSP between number and "%" in some locales).
let r = try {
        Number.ToText(0.5, "P0"),
        Number.ToText(0.5, "P1"),
        Number.ToText(0.5, "P2"),
        Number.ToText(0.5, "P5"),
        Number.ToText(0, "P2"),
        Number.ToText(1, "P0"),
        Number.ToText(1, "P2"),
        Number.ToText(-0.5, "P2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
