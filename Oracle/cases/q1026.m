// Table.RemoveRowsWithErrors with actual cell errors via Table.AddColumn.
let t = Table.FromRecords({[a=1], [a=0], [a=2], [a=0]}) in
// IntegerDivide(1, 0) → null (not error), so use error explicitly.
let withErrs = Table.AddColumn(t, "div", each
    if [a] = 0 then error "divide-by-zero" else 1 / [a]) in
let r = try {
        Table.RemoveRowsWithErrors(withErrs)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
