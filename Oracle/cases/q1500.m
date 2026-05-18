let r = try Table.RowCount(Table.ReplaceKeys(Table.FromRecords({[a=1]}), {})) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
