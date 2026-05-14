let r = try List.Accumulate({1,2,3}, 0,
    (s,c) => if c = 2 then error "boom" else s + c) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
