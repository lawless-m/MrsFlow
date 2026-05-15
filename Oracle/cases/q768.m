// Text.Split with empty separator.
// mrsflow's note says PQ returns one-char list; verify.
let r = try {
        Text.Split("abc", ""),
        Text.Split("", ""),
        Text.Split("a", ""),
        Text.Split("→é!", "")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
