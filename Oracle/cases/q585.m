let r = try {
        Table.RowCount(Table.Distinct(#table({"k"}, {{"a"}, {"a"}, {"a"}}))),
        Table.RowCount(Table.Distinct(#table({"k"}, {}))),
        Table.RowCount(Table.Distinct(#table({"k"}, {{"a"}, {"b"}, {"c"}})))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
