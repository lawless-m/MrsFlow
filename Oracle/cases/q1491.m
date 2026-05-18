let r = try Function.From(type function (s as text) as text, each Text.Upper(_{0})) in
                if r[HasError] then [HasError=true, Message=r[Error][Message]] else [HasError=false, IsFunc=Value.Is(r[Value], type function)]
