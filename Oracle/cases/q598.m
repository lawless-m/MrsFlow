let r = try Table.FillDown(
        #table({"a", "b", "c"}, {{"X", null, 1}, {null, "Q", 2}, {null, null, 3}}),
        {"a", "b"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
