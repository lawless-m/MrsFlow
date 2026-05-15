// Text.Range null inputs.
let r = try {
        Text.Range(null, 0),
        Text.Range("abc", null),
        Text.Range("abc", 0, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
