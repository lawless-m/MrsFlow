let r = try Table.Join(
        #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
        "k",
        #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
        "kr",
        JoinKind.LeftAnti
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
