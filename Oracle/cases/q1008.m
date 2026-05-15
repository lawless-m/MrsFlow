// Table.ReplaceErrorValues — replace per-cell errors.
let t = Table.FromRecords({[a=1], [a=0], [a=2]}) in
// Add a column whose computation errors at row 2 (1/0 → Inf in PQ,
// but Number.IntegerDivide(1, 0) → error). Then replace the errors.
let withErrs = Table.AddColumn(t, "div", each Number.IntegerDivide(1, [a])) in
let r = try {
        Table.ReplaceErrorValues(withErrs, {{"div", -1}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
