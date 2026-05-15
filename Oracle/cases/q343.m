let r = try Table.RowCount(Table.Buffer(#table({"a"}, {{1}, {2}, {3}}))) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
