// Table.FillDown on empty table + missing column.
let empty = Table.FromRecords({}) in
let t = Table.FromRecords({[a=1]}) in
let r = try {
        Table.FillDown(empty, {"a"}),
        Table.FillDown(t, {"missing"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
