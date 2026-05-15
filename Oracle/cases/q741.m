// Sign handling + scientific notation.
let r = try {
        Number.FromText("-42"),
        Number.FromText("+42"),
        Number.FromText("-3.14"),
        Number.FromText("1e5"),
        Number.FromText("1E5"),
        Number.FromText("1e+05"),
        Number.FromText("1e-5"),
        Number.FromText("-1.5e10"),
        Number.FromText("1.5E-10")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
