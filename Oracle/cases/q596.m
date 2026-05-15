let r = try Table.FillDown(
        #table({"a", "b"}, {{"X", 1}, {null, 2}, {null, 3}, {"Y", 4}, {null, 5}}),
        {"a"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
