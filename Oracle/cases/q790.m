// Text.Range offset=length isolated — does PQ return "" or error?
let r = try {
        Text.Range("abc", 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
