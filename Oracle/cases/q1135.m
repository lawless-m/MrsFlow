// Text.Replace with Replacer arg (PQ ignores Replacer fn arg, refuses 4-arg form for some — verify behaviour).
let r = try {
        Text.Replace("hello world", "world", "M"),
        Replacer.ReplaceText("hello world world", "world", "M")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
