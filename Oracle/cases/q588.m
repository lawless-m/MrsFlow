let r = try Table.Sort(
        #table({"g", "v"}, {{"a", 3}, {"b", 1}, {"a", 1}, {"b", 2}, {"a", 2}}),
        {{"g", Order.Ascending}, {"v", Order.Ascending}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
