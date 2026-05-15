// Table.AddIndexColumn defaults — 0-based, step 1.
let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
let r = try {
        Table.AddIndexColumn(t, "Index"),
        Table.AddIndexColumn(t, "Idx", 1),
        Table.AddIndexColumn(t, "Idx", 100, 10)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
