// Text.PadStart/PadEnd null inputs.
let r = try {
        Text.PadStart(null, 5),
        Text.PadStart("a", null),
        Text.PadEnd(null, 5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
