// List.Buffer null / non-list args.
let r = try {
        List.Buffer(null),
        List.Buffer("abc"),
        List.Buffer(42)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
