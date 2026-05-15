// Table.NestedJoin multi-column key.
let a = Table.FromRecords({
        [g="x", k=1, v="a1"],
        [g="x", k=2, v="a2"],
        [g="y", k=1, v="a3"]
    }) in
let b = Table.FromRecords({
        [g="x", k=1, w="b1"],
        [g="x", k=2, w="b2"]
    }) in
let r = try {
        Table.NestedJoin(a, {"g", "k"}, b, {"g", "k"}, "nested", JoinKind.LeftOuter)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
