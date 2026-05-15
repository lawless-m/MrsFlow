let r = try Table.Distinct(
        #table({"k"}, {{"A"}, {"a"}, {"B"}, {"b"}}),
        {"k", Comparer.OrdinalIgnoreCase}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
