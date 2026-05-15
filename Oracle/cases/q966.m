// Table.AddIndexColumn step = 0.
let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
let r = try {
        Table.AddIndexColumn(t, "Index", 5, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
