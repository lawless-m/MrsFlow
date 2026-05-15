// Same-call within try block — does Number.Random get re-evaluated?
let r = try {
        // Two calls in sequence — should be distinct.
        Number.Random() <> Number.Random(),
        // Type check.
        Number.Random() >= 0,
        // .NET Random.NextDouble has 2^32 distinct outputs;
        // two calls colliding is ~2^-32 — call it false in practice.
        Number.RandomBetween(0, 1) <> Number.RandomBetween(0, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
