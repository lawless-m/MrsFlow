// List.Zip identity / inverse.
let pairs = List.Zip({{1, 2, 3}, {"a", "b", "c"}}) in
let r = try {
        List.Count(pairs) = 3,
        List.Count(pairs{0}) = 2,
        // Re-zip — should round-trip.
        List.Zip(List.Zip({{1, 2}, {"a", "b"}})) = {{1, 2}, {"a", "b"}}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
