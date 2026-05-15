// Table.ExpandRecordColumn — missing column or non-record cell.
let t = Table.FromRecords({[k=1, r=[a=10]]}) in
let r = try {
        Table.ExpandRecordColumn(t, "missing", {"a"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
