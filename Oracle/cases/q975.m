// Table.ExpandRecordColumn nested record cell.
let t = Table.FromRecords({
        [k=1, r=[a=[x=1, y=2]]],
        [k=2, r=[a=[x=3, y=4]]]
    }) in
let r = try {
        Table.ExpandRecordColumn(t, "r", {"a"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
