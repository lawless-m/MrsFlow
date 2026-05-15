let r = try {
        Text.Insert("hello", 0, "X"),
        Text.Insert("hello", 5, "X"),
        Text.Insert("hello", 2, ""),
        Text.Insert("", 0, "abc")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
