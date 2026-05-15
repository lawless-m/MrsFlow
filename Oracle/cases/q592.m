let r = try Table.SelectRows(
        #table({"a", "b"}, {{1, 10}, {2, 20}, {3, 30}, {4, 40}, {5, 50}}),
        each [a] > 1 and [b] < 40
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
