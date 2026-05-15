let r = try {
        Table.RowCount(#table({"a", "b"}, {{1, 2}, {3, 4}, {5, 6}, {7, 8}, {9, 10}})),
        Table.ColumnCount(#table({"a", "b"}, {{1, 2}, {3, 4}})),
        Table.RowCount(#table({}, {})),
        Table.ColumnCount(#table({"only"}, {{1}}))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
