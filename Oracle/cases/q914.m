// List.Buffer identity — same as original after buffering.
let r = try {
        List.Buffer({1, 2, 3}) = {1, 2, 3},
        List.Count(List.Buffer({1, 2, 3})) = 3,
        List.Sum(List.Buffer({10, 20, 30})) = 60,
        // Round-trip through Buffer doesn't change downstream ops.
        List.Transform(List.Buffer({1, 2, 3}), each _ * 2) = {2, 4, 6}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
