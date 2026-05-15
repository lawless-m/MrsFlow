// Text.PositionOfAny — char-set, returns first/all char positions.
let r = try {
        Text.PositionOfAny("hello world", {"l", "o"}),
        Text.PositionOfAny("hello world", {"l", "o"}, Occurrence.All),
        Text.PositionOfAny("hello world", {"l", "o"}, Occurrence.Last),
        Text.PositionOfAny("abc", {"x"}),
        Text.PositionOfAny("abc", {"x"}, Occurrence.All)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
