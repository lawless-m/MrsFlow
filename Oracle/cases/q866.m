// List.Accumulate with null seed.
let r = try {
        List.Accumulate({1, 2, 3}, null, (acc, x) => if acc = null then x else acc + x),
        List.Accumulate({}, null, (acc, x) => x)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
