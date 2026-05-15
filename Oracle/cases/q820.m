// Text.PositionOf with empty needle — PQ returns 0 (matches at position 0).
let r = try {
        Text.PositionOf("abc", ""),
        Text.PositionOf("", ""),
        Text.PositionOf("", "a"),
        Text.PositionOf("abc", "", Occurrence.All),
        Text.PositionOf("abc", "", Occurrence.Last)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
