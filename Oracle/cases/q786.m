// Text.Range with unicode (PQ counts characters not bytes).
let r = try {
        Text.Range("café", 0, 3),
        Text.Range("café", 1, 2),
        Text.Range("café", 3, 1),
        Text.Range("→→→", 0, 2),
        Text.Range("naïve", 2, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
