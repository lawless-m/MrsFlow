let r = try Table.ColumnNames(Table.Schema(#table({"a","b"}, {{1,2}}))) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
