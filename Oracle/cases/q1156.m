// Negative-zero and very small / large negative numbers.
let r = try {
        Text.From(-0),
        Text.From(0),
        Text.From(-9007199254740992),
        Json.FromValue(-9007199254740992),
        Json.FromValue(-1e15)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
