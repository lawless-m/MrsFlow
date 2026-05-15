// Table.ReplaceErrorValues per-cell with Replacer.ReplaceValue semantics.
// Build a table where one cell IS an error and replace it.
let t = Table.FromRecords({[a=1, b=1], [a=2, b=2]}) in
let t2 = Table.AddColumn(t, "c", each if [a] = 2 then error "boom" else [a] * 10) in
let r = try {
        Table.ReplaceErrorValues(t2, {{"c", -1}}),
        Table.RemoveRowsWithErrors(t2, {"c"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
