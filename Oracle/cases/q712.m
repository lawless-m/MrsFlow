// C default culture (corpus runs en-GB, expect £).
let r = try {
        Number.ToText(1234.5, "C"),
        Number.ToText(1234.5, "C0"),
        Number.ToText(1234.5, "C2"),
        Number.ToText(1234.5, "C4"),
        Number.ToText(0, "C2"),
        Number.ToText(0.5, "C2"),
        Number.ToText(-1234.5, "C2"),
        Number.ToText(-0.5, "C2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
