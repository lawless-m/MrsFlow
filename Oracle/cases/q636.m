let r = try {
        Table.RowCount(#table({"a"}, {{1}, {2}, {3}})),
        Table.RowCount(#table({"a"}, {})),
        Table.ColumnCount(#table({"a", "b", "c"}, {{1, 2, 3}})),
        Table.ColumnCount(#table({}, {}))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
