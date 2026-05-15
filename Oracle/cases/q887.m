// List.Zip with many lists.
let r = try {
        List.Zip({{1, 2, 3}, {"a", "b", "c"}, {true, false, true}, {10, 20, 30}}),
        List.Zip({{1}}),
        List.Zip({{1, 2}, {"a", "b"}, {}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
