// Table.ExpandRecordColumn basic.
let t = Table.FromRecords({
        [k=1, r=[a=10, b=20]],
        [k=2, r=[a=30, b=40]]
    }) in
let r = try {
        Table.ExpandRecordColumn(t, "r", {"a", "b"}),
        Table.ExpandRecordColumn(t, "r", {"a"}),
        Table.ExpandRecordColumn(t, "r", {"a", "b"}, {"a2", "b2"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
