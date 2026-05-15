let r = try Table.SelectColumns(
        #table({"a", "b", "c", "d"}, {{1, 2, 3, 4}}),
        {"c", "a"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
