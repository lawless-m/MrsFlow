let r = try Table.Distinct(
        #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"a", 1}, {"c", 3}, {"b", 2}})
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
