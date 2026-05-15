// Stay within f64-exact-integer range (≤ 2^53) since mrsflow's numeric
// literals go through f64. PQ uses Decimal internally so it preserves
// 19-digit i64 literals exactly; this probe sidesteps that parser-precision
// divergence.
let r = try {
        Number.BitwiseAnd(9007199254740991, 1),
        Number.BitwiseAnd(9007199254740991, 0xFF),
        Number.BitwiseOr(9007199254740990, 1),
        Number.BitwiseXor(9007199254740991, 9007199254740991),
        Number.BitwiseAnd(0x1FFFFFFFFFFFFF, 0xFFFFFFFF),
        Number.BitwiseOr(0x100000000, 0xFFFFFFFF),
        Number.BitwiseShiftRight(0x1FFFFFFFFFFFFF, 32),
        Number.BitwiseShiftRight(0x1FFFFFFFFFFFFF, 52)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
