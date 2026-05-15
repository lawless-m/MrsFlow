// Subnormal / smallest representable values.
let r = try {
        Number.Abs(4.9e-324),
        Number.Abs(-4.9e-324),
        Number.Sign(4.9e-324),
        Number.Sign(-4.9e-324),
        Number.Abs(2.2250738585072014e-308),
        Number.Sign(2.2250738585072014e-308),
        Number.Sign(1e-323),
        Number.Sign(-1e-323)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
