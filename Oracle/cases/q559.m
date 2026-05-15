let r = try {
        Number.BitwiseShiftLeft(1, 4),
        Number.BitwiseShiftLeft(3, 2),
        Number.BitwiseShiftRight(256, 4),
        Number.BitwiseShiftRight(255, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
