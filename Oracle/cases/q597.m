let r = try Table.FillUp(
        #table({"a", "b"}, {{null, 1}, {null, 2}, {"X", 3}, {null, 4}, {"Y", 5}}),
        {"a"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
