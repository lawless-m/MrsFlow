// Text.Range count = 0; count > remaining; default count (omitted).
let r = try {
        Text.Range("abc", 0, 0),
        Text.Range("abc", 1, 0),
        Text.Range("abc", 0, 5),
        Text.Range("abc", 1, 5),
        Text.Range("abc", 0),
        Text.Range("abc", 1),
        Text.Range("abc", 2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
