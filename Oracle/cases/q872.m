// List.Generate with record state — multi-arg via record.
let r = try {
        List.Generate(
            () => [i=0, sum=0],
            (s) => s[i] < 5,
            (s) => [i=s[i]+1, sum=s[sum]+s[i]],
            (s) => s[sum]
        ),
        List.Generate(
            () => [a=1, b=1],
            (s) => s[a] < 100,
            (s) => [a=s[b], b=s[a]+s[b]],
            (s) => s[a]
        )
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
