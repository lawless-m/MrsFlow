// Zero-padded integer patterns.
let r = try {
        Number.ToText(42, "00000"),
        Number.ToText(42, "000"),
        Number.ToText(42, "0"),
        Number.ToText(0, "00000"),
        Number.ToText(-42, "00000"),
        Number.ToText(123456, "00000"),
        Number.ToText(1.5, "00000"),
        Number.ToText(0.5, "0000")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
