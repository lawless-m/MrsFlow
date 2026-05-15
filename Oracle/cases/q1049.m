// Record.Combine — concatenate records; later wins on collision.
let r = try {
        Record.Combine({[a=1], [b=2]}),
        Record.Combine({[a=1, b=2], [b=20, c=30]}),
        Record.Combine({[]}),
        Record.Combine({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
