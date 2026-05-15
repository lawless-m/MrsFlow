let r = try {
        Comparer.OrdinalIgnoreCase("a", "A"),
        Comparer.OrdinalIgnoreCase("a", "B"),
        Comparer.OrdinalIgnoreCase("B", "a"),
        Comparer.OrdinalIgnoreCase("Hello", "HELLO")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
