// Text.Range with negative count (separate so q784 isolates negative offset).
let r = try {
        Text.Range("abc", 0, -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
