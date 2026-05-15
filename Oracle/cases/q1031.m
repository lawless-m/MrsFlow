// Table.RowCount round-trip: build a table, count rows after errors removed.
let t = Table.FromRecords({[a=1], [a=0], [a=2], [a=0], [a=3]}) in
let withErrs = Table.AddColumn(t, "x", each
    if [a] = 0 then error "zero" else [a] * 10) in
let r = try {
        Table.RowCount(Table.RemoveRowsWithErrors(withErrs)) = 3
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
