// Text.Split with whole-string delimiter equal to text, plus null inputs.
let r = try {
        Text.Split("abc", "abc"),
        Text.Split("", "x"),
        Text.Split("abc", "b"),
        Text.Split("abcabc", "abc")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
