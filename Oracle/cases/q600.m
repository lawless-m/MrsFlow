let r = try Table.FillDown(
        #table({"a"}, {{"X"}, {"Y"}, {"Z"}}),
        {"a"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
