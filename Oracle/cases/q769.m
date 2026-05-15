// Text.Split with multi-char delimiter.
let r = try {
        Text.Split("a--b--c", "--"),
        Text.Split("a--", "--"),
        Text.Split("--a", "--"),
        Text.Split("----", "--"),
        Text.Split("abc", "xyz")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
