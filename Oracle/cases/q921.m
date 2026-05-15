// Number.RandomBetween bounds.
let r = try {
        Number.RandomBetween(0, 1) >= 0,
        Number.RandomBetween(0, 1) <= 1,
        Number.RandomBetween(-10, 10) >= -10,
        Number.RandomBetween(-10, 10) <= 10,
        Number.RandomBetween(100, 100) = 100
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
