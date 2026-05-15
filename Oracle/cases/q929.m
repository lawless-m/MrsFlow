// List.Transform with side-effect via List.Generate index tracking.
let r = try {
        List.Transform({100, 200, 300}, (v, i) => [pos=i, val=v]),
        List.Transform(List.Numbers(1, 5), (n, i) => n * 100 + i)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
