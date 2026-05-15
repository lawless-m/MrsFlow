// List.Accumulate with type-changing accumulator (text → number → text).
// Tests that mrsflow doesn't constrain acc type.
let r = try {
        List.Accumulate({1, 2, 3}, "start", (acc, x) =>
            if acc = "start" then 0 else acc + x),
        List.Accumulate({"a", "b"}, 0, (acc, x) =>
            if acc = 0 then x else Text.From(acc) & x)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
