// List.Accumulate with list seed — building a list result.
let r = try {
        List.Accumulate({1, 2, 3}, {}, (acc, x) => acc & {x * 10}),
        List.Accumulate({"a", "b"}, {"start"}, (acc, x) => acc & {x})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
