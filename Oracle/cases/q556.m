let r = try {
        Number.BitwiseAnd(12, 10),
        Number.BitwiseAnd(255, 240),
        Number.BitwiseAnd(0, 0),
        Number.BitwiseAnd(-1, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
