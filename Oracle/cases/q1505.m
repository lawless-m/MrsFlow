let r = try Table.RowCount(Table.FilterWithDataTable(
                Table.FromRecords({[a=1],[a=2],[a=3]}), Table.FromRecords({[a=1],[a=3]}))) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
