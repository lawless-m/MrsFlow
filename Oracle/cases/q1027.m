// Table.RemoveRowsWithErrors on empty table.
let t = Table.FromRecords({}) in
let r = try {
        Table.RemoveRowsWithErrors(t)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
