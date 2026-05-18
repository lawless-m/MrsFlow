let r = try Table.ViewFunction(Table.FromRecords({[a=1]})) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, IsFunc=Value.Is(r[Value], type function)]
