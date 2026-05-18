let r = try Table.ViewError([Reason="x",Message="m",Detail=null]) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, IsTable=Value.Is(r[Value], type table)]
