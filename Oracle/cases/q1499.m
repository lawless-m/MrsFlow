let r = try Type.IsNullable(Type.ReplaceTablePartitionKey(type table [a=number], null)) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, Value=r[Value]]
