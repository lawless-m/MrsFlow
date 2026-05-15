// Text.Insert null inputs.
let r = try {
        Text.Insert(null, 0, "X"),
        Text.Insert("abc", null, "X"),
        Text.Insert("abc", 0, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
