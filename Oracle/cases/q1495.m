let r = try Table.WithErrorContext(Table.FromRecords({[a=1]}), "ctx") in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, RowCount=Table.RowCount(r[Value])]
