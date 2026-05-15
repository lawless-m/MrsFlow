// Chained & with mixed types: text & list errors, list & null propagates null.
let r = try {
        {1, 2} & {3} & {4, 5},
        {1} & null,
        null & {2},
        {1, "a", true} & {null, {2}}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
