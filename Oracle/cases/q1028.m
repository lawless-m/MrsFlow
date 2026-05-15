// Table.RemoveRowsWithErrors when ALL rows have errors.
let t = Table.FromRecords({[a=0], [a=0], [a=0]}) in
let withErrs = Table.AddColumn(t, "div", each
    if [a] = 0 then error "zero" else 1) in
let r = try {
        Table.RemoveRowsWithErrors(withErrs)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
