let r = try {
        Text.PositionOfAny("hello world", {"l", "o"}),
        Text.PositionOfAny("hello world", {"z", "y"}),
        Text.PositionOfAny("hello world", {"o", "l"}, Occurrence.All)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
