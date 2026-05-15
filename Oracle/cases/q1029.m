// Table.RemoveRowsWithErrors with specific column list (2-arg form) —
// pin the selected column to one that has errors, so the kept rows are
// serialisable.
let t = Table.FromRecords({[a=1], [a=0], [a=2], [a=0]}) in
let withErrs = Table.AddColumn(t, "da", each
    if [a] = 0 then error "a-zero" else [a] * 10) in
let r = try {
        Table.RemoveRowsWithErrors(withErrs, {"da"})
            = Table.RemoveRowsWithErrors(withErrs)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
