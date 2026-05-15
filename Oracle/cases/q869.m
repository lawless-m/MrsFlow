// List.Accumulate null inputs.
let r = try {
        List.Accumulate(null, 0, (acc, x) => acc + x),
        List.Accumulate({1, 2}, 0, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
