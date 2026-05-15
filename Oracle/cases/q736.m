// Percent in custom pattern.
let r = try {
        Number.ToText(0.5, "0.00%"),
        Number.ToText(0.5, "0%"),
        Number.ToText(0.123, "0.00%"),
        Number.ToText(0.123, "0.0%"),
        Number.ToText(0, "0.00%"),
        Number.ToText(-0.5, "0.00%"),
        Number.ToText(1, "0.00%")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
