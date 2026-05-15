// Text.Insert with negative offset.
let r = try {
        Text.Insert("abc", -1, "X")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
