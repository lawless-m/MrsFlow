// Text.Insert offset > length, negative, fractional.
let r = try {
        Text.Insert("abc", 4, "X")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
