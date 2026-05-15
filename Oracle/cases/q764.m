// Text.Replace multi-occurrence + overlapping needles.
// PQ scans left-to-right and doesn't re-scan inserted text.
let r = try {
        Text.Replace("aaaa", "aa", "b"),
        Text.Replace("ababab", "ab", "X"),
        Text.Replace("aaa", "aa", "a"),
        Text.Replace("xx", "x", "xx")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
