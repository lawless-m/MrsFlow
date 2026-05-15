// List.Generate selector returning various types.
let r = try {
        List.Generate(() => 0, (s) => s < 3, (s) => s + 1, (s) => [n=s, doubled=s*2]),
        List.Generate(() => 0, (s) => s < 3, (s) => s + 1, (s) => {s, s+1}),
        List.Generate(() => 0, (s) => s < 3, (s) => s + 1, (s) => null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
