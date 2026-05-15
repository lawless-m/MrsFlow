// Text.Lower/Upper null input — does PQ propagate or error?
let r = try {
        Text.Upper(null),
        Text.Lower(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
