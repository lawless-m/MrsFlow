// Text.PadStart/PadEnd defaults — pad char defaults to space.
let r = try {
        Text.PadStart("a", 5),
        Text.PadEnd("a", 5),
        Text.PadStart("", 3),
        Text.PadEnd("", 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
