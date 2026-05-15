// Cross-call uniqueness — two consecutive calls should differ.
let a = Text.NewGuid(), b = Text.NewGuid() in
let r = try {
        a <> b,
        Text.Length(a) = Text.Length(b)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
