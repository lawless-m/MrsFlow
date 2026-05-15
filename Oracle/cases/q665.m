let r = try {
        Number.Mod(1, 5),
        Number.Mod(-1, 5),
        Number.Mod(1, -5),
        Number.Mod(-1, -5),
        Number.Mod(5, 7),
        Number.Mod(-5, 7),
        Number.Mod(5, -7),
        Number.Mod(-5, -7)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
