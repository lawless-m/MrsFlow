// Edge values near i64 bounds. i64::MAX is 9223372036854775807;
// f64 can represent ~2^53 = 9007199254740992 exactly. Beyond that,
// integer-divide should still produce a well-defined floor.
let r = try {
        Number.IntegerDivide(9007199254740992, 2),
        Number.IntegerDivide(-9007199254740992, 2),
        Number.IntegerDivide(9007199254740992, -2),
        Number.IntegerDivide(-9007199254740992, -2),
        Number.IntegerDivide(9007199254740992, 9007199254740992),
        Number.IntegerDivide(9223372036854775000, 1),
        Number.IntegerDivide(-9223372036854775000, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
