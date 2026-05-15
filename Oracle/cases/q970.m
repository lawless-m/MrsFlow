// Table.ExpandRecordColumn with null cells + missing fields.
let t = Table.FromRecords({
        [k=1, r=[a=10, b=20]],
        [k=2, r=null],
        [k=3, r=[a=30]]
    }) in
let r = try {
        Table.ExpandRecordColumn(t, "r", {"a", "b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
