// Text.Replace with empty old → expected no-op (str::replace would
// otherwise insert `new` between every char and at both ends).
let r = try {
        Text.Replace("abc", "", "X"),
        Text.Replace("", "", "X"),
        Text.Replace("abc", "", ""),
        Text.Replace("", "abc", "X")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
