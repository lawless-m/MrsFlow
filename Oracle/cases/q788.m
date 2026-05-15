// Text.Range fractional offset / count.
let r = try {
        Text.Range("abc", 1.5),
        Text.Range("abc", 0, 1.5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
