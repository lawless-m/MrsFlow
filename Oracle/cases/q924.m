// Number.RandomBetween with reversed bounds, equal bounds, negatives.
let r = try {
        Number.RandomBetween(10, 0),
        Number.RandomBetween(5, 5),
        Number.RandomBetween(-1, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
