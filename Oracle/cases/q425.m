let r = try Table.Join(
        #table({"k1", "k2", "v"}, {{"a", 1, "X"}, {"a", 2, "Y"}, {"b", 1, "Z"}}),
        {"k1", "k2"},
        #table({"kr1", "kr2", "w"}, {{"a", 1, 100}, {"a", 2, 200}, {"c", 1, 300}}),
        {"kr1", "kr2"},
        JoinKind.Inner
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
