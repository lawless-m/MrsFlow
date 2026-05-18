let r = try Value.ViewError([Reason="x",Message="m",Detail=null]) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, IsRecord=Value.Is(r[Value], type record)]
