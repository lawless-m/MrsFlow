// N small numbers don't get thousand separators.
let r = try {
        Number.ToText(999, "N0"),
        Number.ToText(1000, "N0"),
        Number.ToText(9999, "N0"),
        Number.ToText(10000, "N0"),
        Number.ToText(100000, "N0"),
        Number.ToText(999.99, "N2"),
        Number.ToText(1000.99, "N2"),
        Number.ToText(-999.99, "N2"),
        Number.ToText(-1000.99, "N2")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
