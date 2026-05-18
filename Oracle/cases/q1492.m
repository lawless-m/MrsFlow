let r = try Type.IsNullable(Type.ForFunction([ReturnType=type number, Parameters=[a=type number]], 1)) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
