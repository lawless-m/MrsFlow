let r = try {
        Number.Mod(10, 3),
        Number.Mod(11, 3),
        Number.Mod(12, 3),
        Number.Mod(100, 7),
        Number.Mod(-100, 7),
        Number.Mod(100, -7),
        Number.Mod(-100, -7),
        Number.Mod(1000000, 17)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
