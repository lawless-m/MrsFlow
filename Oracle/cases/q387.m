let r = try {
        Value.Equals(1, "1"),
        Value.Equals(true, 1),
        Value.Equals({1, 2}, {1, 2}),
        Value.Equals({1, 2}, {2, 1}),
        Value.Equals([a=1, b=2], [b=2, a=1])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
