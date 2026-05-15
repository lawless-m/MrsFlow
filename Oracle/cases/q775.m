// Text.PadStart width = 0 / less-than-len / equal-to-len → no padding.
let r = try {
        Text.PadStart("abc", 0),
        Text.PadStart("abc", 1),
        Text.PadStart("abc", 2),
        Text.PadStart("abc", 3),
        Text.PadStart("", 0),
        Text.PadStart("", 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
