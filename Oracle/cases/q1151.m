// Text.From across the 1e15 → 1e16 threshold. PQ retains Decimal
// precision for some integer literals; mrsflow rounds via f64. We test
// values that survive f64 round-trip exactly: powers of 10 and 2^53.
let r = try {
        Text.From(999999999999999),
        Text.From(1000000000000000),
        Text.From(9007199254740992),
        Text.From(10000000000000000),
        Text.From(-1000000000000000),
        Text.From(-10000000000000000)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
