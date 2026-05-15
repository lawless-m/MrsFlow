// Text.PositionOf needle = text, needle > text length.
let r = try {
        Text.PositionOf("abc", "abc"),
        Text.PositionOf("abc", "abcd"),
        Text.PositionOf("a", "ab"),
        Text.PositionOf("abc", "abc", Occurrence.All)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
