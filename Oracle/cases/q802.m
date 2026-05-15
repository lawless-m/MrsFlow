// Text.Trim with multi-char elements in list — mrsflow flattens to char set;
// does PQ accept that or require single-char strings?
let r = try {
        Text.Trim("ABabcBA", {"AB"}),
        Text.Trim("abc", {"abc"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
