// Null inputs for Number.RandomBetween / List.Random.
let r = try {
        Number.RandomBetween(null, 10),
        Number.RandomBetween(0, null),
        List.Random(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
