let r = try {
        Number.Mod(0.1, 0.03),
        Number.Mod(-0.1, 0.03),
        Number.Mod(1.1, 1),
        Number.Mod(-1.1, 1),
        Number.Mod(1, 1.1),
        Number.Mod(-1, 1.1),
        Number.Mod(0.0001, 0.0001),
        Number.Mod(0.0002, 0.0001)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
