// Number.Sign basic cases.
let r = try {
        Number.Sign(5),
        Number.Sign(-5),
        Number.Sign(0),
        Number.Sign(0.0),
        Number.Sign(-0.0),
        Number.Sign(0.001),
        Number.Sign(-0.001),
        Number.Sign(1e-300),
        Number.Sign(-1e-300)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
