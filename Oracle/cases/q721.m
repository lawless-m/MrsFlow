// D with large/edge integers.
let r = try {
        Number.ToText(2147483647, "D"),
        Number.ToText(2147483647, "D15"),
        Number.ToText(-2147483648, "D15"),
        Number.ToText(9223372036854775000, "D"),
        Number.ToText(-9223372036854775000, "D"),
        Number.ToText(0, "D0"),
        Number.ToText(0, "D10"),
        Number.ToText(1000000, "D0"),
        Number.ToText(1000000, "D10")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
