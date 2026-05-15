let r = try {
        Character.ToNumber("A"),
        Character.ToNumber("a"),
        Character.ToNumber("0"),
        Character.ToNumber(" ")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
