// Text.Replace with built-in Comparer.OrdinalIgnoreCase as 4th arg.
// Does PQ even accept a 4th arg here? Text.Replace docs say 3-arg only.
let r = try {
        Text.Replace("Hello hello HELLO", "hello", "X", Comparer.OrdinalIgnoreCase),
        Text.Replace("Hello", "HELLO", "X", Comparer.OrdinalIgnoreCase),
        Text.Replace("Hello", "hello", "X", Comparer.Ordinal),
        Text.Replace("Hello", "HELLO", "X", Comparer.Ordinal)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
