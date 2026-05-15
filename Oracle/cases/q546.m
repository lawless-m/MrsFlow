let r = try {
        Number.Sin(0),
        Number.Cos(0),
        Number.Tan(0),
        Number.Sin(1.5707963267948966)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
