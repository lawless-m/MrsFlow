// BitwiseOr / BitwiseXor / BitwiseNot.
let r = try {
        Number.BitwiseOr(0xF0, 0x0F),
        Number.BitwiseOr(-1, 0),
        Number.BitwiseOr(0, -1),
        Number.BitwiseXor(0xFF, 0xFF),
        Number.BitwiseXor(0xFF, 0x00),
        Number.BitwiseXor(-1, -1),
        Number.BitwiseXor(-1, 0),
        Number.BitwiseNot(0),
        Number.BitwiseNot(-1),
        Number.BitwiseNot(0xFF)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
