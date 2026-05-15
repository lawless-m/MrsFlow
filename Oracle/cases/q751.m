// Sign × Abs identity: Sign(x) * Abs(x) = x for finite non-zero values.
let r = try {
        Number.Sign(42) * Number.Abs(42),
        Number.Sign(-42) * Number.Abs(-42),
        Number.Sign(3.14) * Number.Abs(3.14),
        Number.Sign(-3.14) * Number.Abs(-3.14),
        Number.Sign(0) * Number.Abs(0),
        Number.Sign(1e10) * Number.Abs(1e10),
        Number.Sign(-1e-10) * Number.Abs(-1e-10)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
