// List.Skip with negative count.
let r = try {
        List.Skip({10, 20, 30}, -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
