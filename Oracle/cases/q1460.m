let
                v = Binary.View(#binary({1,2,3}), [GetLength = () => 3]),
                r = try Binary.ViewFunction(v) in
                    if r[HasError] then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, IsFunc=Value.Is(r[Value], type function)]
