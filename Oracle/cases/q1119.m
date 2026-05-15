// Comparer.Ordinal vs Comparer.OrdinalIgnoreCase on culture-sensitive pairs.
// Comparer.* returns -1/0/1.
let r = try {
        Comparer.Ordinal("ß", "ss"),
        Comparer.OrdinalIgnoreCase("ß", "ss"),
        Comparer.Ordinal("é", "e"),
        Comparer.OrdinalIgnoreCase("É", "é"),
        Comparer.Ordinal("İ", "i"),
        Comparer.OrdinalIgnoreCase("İ", "i")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
