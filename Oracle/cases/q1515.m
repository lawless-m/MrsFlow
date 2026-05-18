let r = try Table.RowCount(Excel.ShapeTable(Table.FromRecords({[a=1],[a=2]}))) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
