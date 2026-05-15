// Identity / composition: Combine and Split are inverses (with single-char delim).
let r = try {
        Text.Combine(Text.Split("a,b,c", ","), ",") = "a,b,c",
        Text.Combine(Text.Split("a-b-c", "-"), "-") = "a-b-c",
        Text.Combine({"a"}, "anything") = "a"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
