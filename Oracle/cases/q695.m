// Large and small magnitude with F precision.
let r = try {
        Number.ToText(1234567.89, "F2"),
        Number.ToText(0.0001, "F4"),
        Number.ToText(0.0001, "F2"),
        Number.ToText(1e10, "F2"),
        Number.ToText(1e-10, "F12"),
        Number.ToText(123456789012345, "F0"),
        Number.ToText(-1234567.89, "F2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
