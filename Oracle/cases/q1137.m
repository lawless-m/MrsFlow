// Cell error survives ExpandRecordColumn — error stays at the expanded
// position, not at the outer record.
let t = Table.FromRecords({[id=1, r=[a=10, b=20]], [id=2, r=[a=30, b=40]]}) in
let t2 = Table.AddColumn(t, "r2", each if [id] = 2 then error "boom" else [r]) in
let r = try {
        Table.ColumnNames(Table.ExpandRecordColumn(t, "r", {"a", "b"})),
        Table.ReplaceErrorValues(t2, {{"r2", null}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
