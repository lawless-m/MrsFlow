let r = try Table.SelectRows(
        #table({"n"}, {{1}, {2}, {3}, {4}, {5}}),
        (row) => Number.Mod(row[n], 2) = 0
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
