// List.Transform null inputs.
let r = try {
        List.Transform(null, each _),
        List.Transform({1, 2}, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
