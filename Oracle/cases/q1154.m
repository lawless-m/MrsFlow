// Text.From with explicit fractional values near boundary.
let r = try {
        Text.From(999999999999999.5),
        Text.From(1000000000000000.5),
        Text.From(0.5),
        Text.From(1e15),
        Text.From(1e16)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
