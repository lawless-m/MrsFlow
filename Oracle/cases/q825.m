// Text.PositionOfAny with multi-char list element — should be refused
// (just like Text.Trim's char-list).
let r = try {
        Text.PositionOfAny("abc", {"ab"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
