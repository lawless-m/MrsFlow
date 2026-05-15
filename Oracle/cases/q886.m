// List.Zip mismatched lengths — does PQ pad with null to longest?
let r = try {
        List.Zip({{1, 2, 3}, {"a", "b"}}),
        List.Zip({{1}, {"a", "b", "c"}}),
        List.Zip({{1, 2}, {}}),
        List.Zip({{}, {1, 2}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
