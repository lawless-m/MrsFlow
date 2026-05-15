// List.MaxN / MinN with N negative.
let r = try {
        List.MaxN({1, 2, 3}, -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
