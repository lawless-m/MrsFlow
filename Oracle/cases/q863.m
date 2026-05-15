// List.Accumulate basic — sum, concat, build list.
let r = try {
        List.Accumulate({1, 2, 3, 4}, 0, (acc, x) => acc + x),
        List.Accumulate({1, 2, 3}, 1, (acc, x) => acc * x),
        List.Accumulate({"a", "b", "c"}, "", (acc, x) => acc & x),
        List.Accumulate({}, 100, (acc, x) => acc + x)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
