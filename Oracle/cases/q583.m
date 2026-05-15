let r = try Table.Distinct(
        #table({"a", "b", "c"}, {{"x", 1, 10}, {"x", 1, 20}, {"y", 1, 10}}),
        {"a", "b"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
