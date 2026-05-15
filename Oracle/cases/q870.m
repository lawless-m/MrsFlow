// List.Generate basic — count up; condition halts.
let r = try {
        List.Generate(() => 0, (s) => s < 5, (s) => s + 1),
        List.Generate(() => 1, (s) => s < 100, (s) => s * 2),
        List.Generate(() => 0, (s) => s < 0, (s) => s + 1),
        List.Generate(() => "", (s) => Text.Length(s) < 3, (s) => s & "a")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
