// Table.AddIndexColumn empty table + collision.
let empty = Table.FromRecords({}) in
let t = Table.FromRecords({[Index=99]}) in
let r = try {
        Table.AddIndexColumn(empty, "Index"),
        Table.AddIndexColumn(t, "Index")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
