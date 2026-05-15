// Table.Distinct with column-list — dedupe by subset.
let t = Table.FromRecords({
        [k=1, v="A", w="x"],
        [k=2, v="B", w="y"],
        [k=1, v="A", w="DIFFERENT"]
    }) in
let r = try {
        // Dedupe by {k, v} only — third row is dup despite w differing.
        Table.Distinct(t, {"k", "v"}),
        // Single column form.
        Table.Distinct(t, "k")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
