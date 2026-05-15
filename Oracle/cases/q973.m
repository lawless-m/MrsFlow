// Table.ExpandTableColumn basic.
let inner1 = Table.FromRecords({[x=1, y=2], [x=3, y=4]}) in
let inner2 = Table.FromRecords({[x=5, y=6]}) in
let t = Table.FromRecords({
        [k=1, tbl=inner1],
        [k=2, tbl=inner2]
    }) in
let r = try {
        Table.ExpandTableColumn(t, "tbl", {"x", "y"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
