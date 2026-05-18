let
                r = try Binary.ViewError([Reason="x", Message="m", Detail=null]) in
                    if r[HasError] then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=Value.Is(r[Value], type binary)]
