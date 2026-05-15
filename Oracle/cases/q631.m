let r = try Table.ReorderColumns(
        #table({"a", "b", "c", "d"}, {{1, 2, 3, 4}}),
        {"c", "a", "d", "b"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
