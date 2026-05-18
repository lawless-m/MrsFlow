let r = try Type.TableSchema(type table [a=number, b=text]) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, IsTable=Value.Is(r[Value], type table)]
