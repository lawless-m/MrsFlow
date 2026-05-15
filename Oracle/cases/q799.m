// Text.Trim with list-of-text char set.
let r = try {
        Text.Trim("XYabcYX", {"X", "Y"}),
        Text.Trim("XXabcXX", {"X"}),
        Text.Trim("abc", {"X"}),
        Text.Trim("", {"X"}),
        Text.Trim("XYXY", {"X", "Y"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
