// Number.Abs basic cases.
let r = try {
        Number.Abs(5),
        Number.Abs(-5),
        Number.Abs(0),
        Number.Abs(0.0),
        Number.Abs(-0.0),
        Number.Abs(3.14),
        Number.Abs(-3.14),
        Number.Abs(1e-300),
        Number.Abs(-1e-300)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
