// Text.Range offset = length, offset > length.
let r = try {
        Text.Range("abc", 3),
        Text.Range("abc", 3, 0),
        Text.Range("abc", 4),
        Text.Range("abc", 4, 0),
        Text.Range("", 0),
        Text.Range("", 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
