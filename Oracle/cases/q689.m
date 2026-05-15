// Number.Exp across normal & boundary values.
let r = try {
        Number.Exp(0),
        Number.Exp(1),
        Number.Exp(-1),
        Number.Exp(10),
        Number.Exp(-10),
        Number.Exp(100),
        Number.Exp(-100),
        Number.Exp(1000),
        Number.Exp(-1000)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
