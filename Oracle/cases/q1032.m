// Table.Distinct basic — no comparer, deduplicates by full row.
let t = Table.FromRecords({
        [k=1, v="A"],
        [k=2, v="B"],
        [k=1, v="A"],
        [k=2, v="C"]
    }) in
let r = try {
        Table.Distinct(t)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
