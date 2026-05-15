let r = try {
        Number.Mod(7.5, 2),
        Number.Mod(-7.5, 2),
        Number.Mod(7.5, -2),
        Number.Mod(-7.5, -2),
        Number.Mod(2.5, 0.5),
        Number.Mod(-2.5, 0.5),
        Number.Mod(2.5, -0.5),
        Number.Mod(-2.5, -0.5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
