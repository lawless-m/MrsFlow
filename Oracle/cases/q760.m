// Identity / algebraic relations.
let r = try {
        Number.BitwiseAnd(255, Number.BitwiseNot(0)) = 255,
        Number.BitwiseXor(123, 123) = 0,
        Number.BitwiseOr(123, 0) = 123,
        Number.BitwiseAnd(123, 0) = 0,
        Number.BitwiseShiftLeft(Number.BitwiseShiftRight(0xFF00, 8), 8) = 0xFF00,
        Number.BitwiseXor(Number.BitwiseAnd(0xF0, 0xCC), Number.BitwiseOr(0xF0, 0xCC)) = Number.BitwiseXor(0xF0, 0xCC),
        Number.BitwiseAnd(5, 3) + Number.BitwiseOr(5, 3) = 5 + 3,
        Number.BitwiseAnd(0xAAAA, 0xFFFF) = 0xAAAA
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
