let r = try {
        Number.Mod(7, 3),
        Number.Mod(-7, 3),
        Number.Mod(7, -3),
        Number.Mod(-7, -3),
        Number.Mod(0, 3),
        Number.Mod(0, -3),
        Number.Mod(3, 3),
        Number.Mod(-3, 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
