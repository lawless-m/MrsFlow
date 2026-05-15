// Null inputs for both PositionOf and PositionOfAny.
let r = try {
        Text.PositionOf(null, "a"),
        Text.PositionOf("abc", null),
        Text.PositionOfAny(null, {"a"}),
        Text.PositionOfAny("abc", null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
