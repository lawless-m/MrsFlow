// Text.PadEnd: empty pad-char (mrsflow errors); negative width; fractional.
let r = try {
        Text.PadEnd("a", 5, ""),
        Text.PadEnd("a", -1),
        Text.PadEnd("a", 1.5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
