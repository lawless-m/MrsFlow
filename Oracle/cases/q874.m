// List.Generate complex termination (multiple state fields).
let r = try {
        List.Generate(
            () => [n=0, found=false],
            (s) => s[n] < 100 and not s[found],
            (s) => [n=s[n]+1, found=Number.Mod(s[n]+1, 7) = 0],
            (s) => s[n]
        )
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
