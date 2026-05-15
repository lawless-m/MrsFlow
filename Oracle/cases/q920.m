// Number.Random — 0 ≤ x < 1; multiple calls produce different values.
let a = Number.Random(), b = Number.Random(), c = Number.Random() in
let r = try {
        a >= 0 and a < 1,
        b >= 0 and b < 1,
        c >= 0 and c < 1,
        // Three random doubles being identical is statistically impossible.
        not (a = b and b = c)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
