let r = try {
        Value.Compare(1, 2),
        Value.Compare(2, 1),
        Value.Compare(1, 1),
        Value.Compare("a", "b"),
        Value.Compare("b", "a")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
