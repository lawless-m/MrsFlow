let r = try {
        Character.FromNumber(8364),
        Character.FromNumber(233),
        Character.FromNumber(9731),
        Character.ToNumber("€")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
