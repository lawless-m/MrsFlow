// Text.PositionOf needle-not-present → -1.
let r = try {
        Text.PositionOf("abc", "z"),
        Text.PositionOf("abc", "z", Occurrence.All),
        Text.PositionOf("abc", "z", Occurrence.Last),
        Text.PositionOfAny("abc", {"x", "y", "z"}),
        Text.PositionOfAny("abc", {"x"}, Occurrence.All)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
