let r = try {
        Value.Equals(1, 1),
        Value.Equals(1, 1.0),
        Value.Equals("a", "a"),
        Value.Equals("a", "A"),
        Value.Equals(null, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
