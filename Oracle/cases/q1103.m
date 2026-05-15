// null op X comparisons.
let r = try {
        null = null,
        null = 0,
        null < 1,
        null > 1,
        1 < null,
        null <> 1
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
