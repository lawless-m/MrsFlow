let r = try {
        Number.Sign(5),
        Number.Sign(-3),
        Number.Sign(0),
        Number.Abs(-7.5),
        Number.Abs(7.5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
