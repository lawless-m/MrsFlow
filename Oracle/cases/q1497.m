let r = try Value.Expression(42) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, IsRecord=Value.Is(r[Value], type record)]
