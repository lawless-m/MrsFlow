// Chained: AddColumn-with-error → ReplaceErrorValues → SelectRows
// (predicate sees the replacement, not the marker).
let t = Table.FromRecords({[a=1], [a=2], [a=3], [a=4]}) in
let t2 = Table.AddColumn(t, "b", each if Number.Mod([a], 2) = 0 then error "even" else [a] * 10) in
let r = try {
        Table.RowCount(Table.ReplaceErrorValues(t2, {{"b", -1}})),
        Table.SelectRows(Table.ReplaceErrorValues(t2, {{"b", -1}}), each [b] > 0),
        Table.SelectRows(Table.ReplaceErrorValues(t2, {{"b", -1}}), each [b] = -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
