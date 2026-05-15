let r = try List.Sort({"banana", "Apple", "cherry", "apple"}, Comparer.OrdinalIgnoreCase) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
