// Text.Replace with empty new → deletion-style replacement.
let r = try {
        Text.Replace("abcabc", "b", ""),
        Text.Replace("aaa", "a", ""),
        Text.Replace("abc", "abc", ""),
        Text.Replace("a-b-c", "-", "")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
