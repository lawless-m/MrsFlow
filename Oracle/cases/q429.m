let r = try Table.NestedJoin(
        #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
        "k",
        #table({"k", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
        "k",
        "Sub",
        JoinKind.Inner
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=Table.ColumnNames(r[Value])]
