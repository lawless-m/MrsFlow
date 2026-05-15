// List.Numbers count=0 / negative / fractional.
let r = try {
        List.Numbers(0, -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
