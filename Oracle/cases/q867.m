// List.Accumulate with deeply-nested seed.
let r = try {
        List.Accumulate({1, 2}, [nested=[count=0, items={}]],
            (acc, x) => [nested=[count=acc[nested][count]+1, items=acc[nested][items]&{x}]])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
