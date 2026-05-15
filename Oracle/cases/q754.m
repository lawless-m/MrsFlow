// BitwiseAnd basic + sign extension.
let r = try {
        Number.BitwiseAnd(0xFF, 0x0F),
        Number.BitwiseAnd(0xFFFF, 0xFF00),
        Number.BitwiseAnd(0, 0xFFFFFFFF),
        Number.BitwiseAnd(-1, -1),
        Number.BitwiseAnd(-1, 0),
        Number.BitwiseAnd(-1, 1),
        Number.BitwiseAnd(0x80000000, 0xFFFFFFFF),
        Number.BitwiseAnd(0xFFFFFFFF, 0xFFFFFFFF)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
