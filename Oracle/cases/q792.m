// Text.Insert empty new, empty target, both empty.
let r = try {
        Text.Insert("abc", 0, ""),
        Text.Insert("abc", 1, ""),
        Text.Insert("abc", 3, ""),
        Text.Insert("", 0, "X"),
        Text.Insert("", 0, "")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
