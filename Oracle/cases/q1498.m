let r = try Type.IsNullable(Type.ReplaceTableKeys(type table [a=number], {})) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
