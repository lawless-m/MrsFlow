let r = try Table.SelectRows(
        #table({"v"}, {{1}, {null}, {3}, {null}, {5}}),
        each [v] <> null
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
