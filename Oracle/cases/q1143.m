// Error in ExpandListColumn source: row with error in list column
// is treated as a single null row or refused.
let t = Table.FromRecords({[id=1, xs={10, 20}], [id=2, xs={30, 40}]}) in
let t2 = Table.AddColumn(t, "ys", each if [id] = 2 then error "bad" else [xs]) in
let r = try {
        Table.ExpandListColumn(t, "xs"),
        Table.RowCount(Table.ExpandListColumn(Table.ReplaceErrorValues(t2, {{"ys", {}}}), "ys"))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
