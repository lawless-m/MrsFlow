// Replacer.ReplaceText called directly: substring-replace semantics.
let r = try {
        Replacer.ReplaceText("hello world", "world", "M"),
        Replacer.ReplaceText("aaa", "a", "bb"),
        Replacer.ReplaceText("hello", "", "x"),
        Replacer.ReplaceText("", "a", "b"),
        Replacer.ReplaceText("résumé", "é", "e")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
