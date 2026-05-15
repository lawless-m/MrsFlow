// Table.Pivot with duplicate (k, attr) pairs — needs aggregator.
let t = Table.FromRecords({
        [k="A", attr="x", val=1],
        [k="A", attr="x", val=10],
        [k="B", attr="x", val=3]
    }) in
let r = try {
        // No aggregator: PQ may error (duplicate key).
        Table.Pivot(t, {"x"}, "attr", "val"),
        // With aggregator (List.Sum):
        Table.Pivot(t, {"x"}, "attr", "val", List.Sum)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
