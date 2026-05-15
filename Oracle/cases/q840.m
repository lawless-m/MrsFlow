// Text.Combine null arg / non-list arg.
let r = try {
        Text.Combine(null),
        Text.Combine("abc")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
