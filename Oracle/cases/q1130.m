// Replacer.ReplaceValue called directly: 3-arg semantics on scalars.
let r = try {
        Replacer.ReplaceValue(5, 5, "five"),
        Replacer.ReplaceValue(5, 4, "five"),
        Replacer.ReplaceValue("a", "a", "b"),
        Replacer.ReplaceValue("hello", "hello", null),
        Replacer.ReplaceValue(null, null, 0),
        Replacer.ReplaceValue(null, 0, "x")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
