// Table.AddColumn on empty table.
let t = Table.FromRecords({}) in
let r = try {
        Table.AddColumn(t, "c", each 1, type number)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
