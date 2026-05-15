// Very large / very small magnitudes, edge precision.
let r = try {
        Number.FromText("1e100"),
        Number.FromText("1e-100"),
        Number.FromText("1e308"),
        Number.FromText("1e-308"),
        Number.FromText("1.7976931348623157e308"),
        Number.FromText("9007199254740992"),
        Number.FromText("9223372036854775807"),
        Number.FromText("0.000000000000000000001"),
        Number.FromText("1234567890123456")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
