// Text.PadStart: multi-char pad string — PQ "isn't a single-character string"?
let r = try {
        Text.PadStart("a", 5, "ab")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
