// List.Random count + seed.
let r = try {
        List.Count(List.Random(10)) = 10,
        List.Count(List.Random(0)) = 0,
        // Seed determinism if supported.
        List.Random(5, 42) = List.Random(5, 42),
        List.Random(5, 42) <> List.Random(5, 99)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
