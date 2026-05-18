let r = try Type.Is(0, BinaryEncoding.Type) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]]
                else [HasError=false, Value=r[Value]]
