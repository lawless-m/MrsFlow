// Text.Range over-count isolated — does PQ truncate or error?
let r = try {
        Text.Range("abc", 0, 5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
