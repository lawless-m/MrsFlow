// C very small + very large magnitudes.
let r = try {
        Number.ToText(0, "C", "en-US"),
        Number.ToText(0.01, "C2", "en-US"),
        Number.ToText(1e-5, "C6", "en-US"),
        Number.ToText(1e10, "C0", "en-US"),
        Number.ToText(1e15, "C0", "en-US"),
        Number.ToText(123456789012345, "C0", "en-US"),
        Number.ToText(-123456789012345, "C0", "en-US")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
