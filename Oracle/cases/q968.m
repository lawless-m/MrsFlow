// Table.AddIndexColumn null/missing-arg handling.
let t = Table.FromRecords({[v=1]}) in
let r = try {
        Table.AddIndexColumn(t, "Index", null, null),
        Table.AddIndexColumn(t, "Index", null, 2),
        Table.AddIndexColumn(t, "Index", 5, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
