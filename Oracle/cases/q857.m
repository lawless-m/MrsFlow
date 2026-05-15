// List.Distinct with Comparer.OrdinalIgnoreCase.
let r = try {
        List.Distinct({"Apple", "apple", "APPLE", "banana"}, Comparer.OrdinalIgnoreCase),
        List.Distinct({"a", "A"}, Comparer.OrdinalIgnoreCase),
        List.Distinct({"a", "A"}, Comparer.Ordinal),
        List.Distinct({"a", "A"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
