let r = try Type.IsNullable(Type.AddTableKey(type table [a=number], {"a"}, true)) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
