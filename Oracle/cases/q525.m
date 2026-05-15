let r = try List.Union({{1, 2, 3}, {2, 3, 4}, {3, 4, 5}}, Comparer.Ordinal) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
