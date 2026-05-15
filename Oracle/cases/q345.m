let r = try Table.ColumnNames(Table.Buffer(#table({"col1", "col2"}, {{1, "a"}, {2, "b"}}))) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
