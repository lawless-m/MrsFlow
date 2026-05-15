// Text.PadEnd same width corners.
let r = try {
        Text.PadEnd("abc", 0),
        Text.PadEnd("abc", 2),
        Text.PadEnd("abc", 3),
        Text.PadEnd("abc", 6, "X"),
        Text.PadEnd("", 0),
        Text.PadEnd("", 3, "*")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
