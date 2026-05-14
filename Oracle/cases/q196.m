let r = try List.Accumulate({1,2,3}, {}, (s,c) => s & {c*2}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
