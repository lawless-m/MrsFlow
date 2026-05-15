// Table.ReplaceErrorValues all-error column.
let t = Table.FromRecords({[a=0], [a=0], [a=0]}) in
let withErrs = Table.AddColumn(t, "div", each Number.IntegerDivide(1, [a])) in
let r = try {
        Table.ReplaceErrorValues(withErrs, {{"div", "ZERO"}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
