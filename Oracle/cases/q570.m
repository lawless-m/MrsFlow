let r = try {
        Text.Length("ÄÖÜß"),
        Text.Length("hello"),
        Character.ToNumber("Ä"),
        Character.ToNumber("ß")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
