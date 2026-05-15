// Text.Insert at boundaries (0, middle, end).
let r = try {
        Text.Insert("abc", 0, "X"),
        Text.Insert("abc", 1, "X"),
        Text.Insert("abc", 2, "X"),
        Text.Insert("abc", 3, "X")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
