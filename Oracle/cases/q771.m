// Text.SplitAny — each char in 2nd arg is its own delimiter.
let r = try {
        Text.SplitAny("a,b;c|d", ",;|"),
        Text.SplitAny("a,,b", ",;"),
        Text.SplitAny("abc", ""),
        Text.SplitAny("", ",;"),
        Text.SplitAny("→é→é", "→é")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
