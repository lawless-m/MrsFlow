// Table.Join multi-column keys.
let a = Table.FromRecords({
        [g="g1", k=1, v="a11"],
        [g="g1", k=2, v="a12"],
        [g="g2", k=1, v="a21"]
    }) in
let b = Table.FromRecords({
        [g="g1", k=1, w="b11"],
        [g="g1", k=2, w="b12"],
        [g="g3", k=1, w="b31"]
    }) in
let r = try {
        Table.Join(a, {"g", "k"}, b, {"g", "k"}, JoinKind.Inner)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
