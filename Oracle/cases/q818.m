// Text.NewGuid with extra args — does PQ refuse or ignore?
let r = try {
        Text.NewGuid("D")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
