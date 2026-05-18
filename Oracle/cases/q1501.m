let r = try Table.RowCount(Table.ReplacePartitionKey(Table.FromRecords({[a=1]}), null)) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
