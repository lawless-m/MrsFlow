let r = try {
        Number.Mod(7.5, 2),
        Number.Mod(10, 2.5),
        Number.Mod(-7.5, 2),
        Number.Mod(7.5, -2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
