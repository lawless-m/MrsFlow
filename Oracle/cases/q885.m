// List.Zip equal-length lists — basic pairing.
let r = try {
        List.Zip({{1, 2, 3}, {"a", "b", "c"}}),
        List.Zip({{1, 2}, {"a", "b"}, {true, false}}),
        List.Zip({{}}),
        List.Zip({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
