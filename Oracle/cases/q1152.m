// Number.ToText with no format around the boundary. Stays within
// f64-exact values to avoid the Decimal-vs-f64 representation gap PQ
// retains for some literals (see q1151 note).
let r = try {
        Number.ToText(999999999999999),
        Number.ToText(1000000000000000),
        Number.ToText(9007199254740992),
        Number.ToText(9007199254740994)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
