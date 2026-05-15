let r = try Table.Distinct(
        #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}, {"a", 1}}),
        "k"
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
