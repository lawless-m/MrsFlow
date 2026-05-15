// Shift left/right basics + larger shift counts.
let r = try {
        Number.BitwiseShiftLeft(1, 0),
        Number.BitwiseShiftLeft(1, 1),
        Number.BitwiseShiftLeft(1, 8),
        Number.BitwiseShiftLeft(1, 30),
        Number.BitwiseShiftLeft(1, 31),
        Number.BitwiseShiftLeft(1, 62),
        Number.BitwiseShiftLeft(1, 63),
        Number.BitwiseShiftRight(256, 8),
        Number.BitwiseShiftRight(0xFFFFFFFF, 16),
        Number.BitwiseShiftRight(-1, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
