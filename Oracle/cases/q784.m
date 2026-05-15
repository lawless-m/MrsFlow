// Text.Range with negative offset / count.
let r = try {
        Text.Range("abc", -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
