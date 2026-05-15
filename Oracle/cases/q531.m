let r = try {
        Character.FromNumber(65),
        Character.FromNumber(97),
        Character.FromNumber(48),
        Character.FromNumber(32)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
