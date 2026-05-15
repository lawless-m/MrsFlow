// Table.Sort edge cases: empty / single-row / missing column.
let t = Table.FromRecords({[a=1, b=2]}) in
let r = try {
        Table.Sort(Table.FromRecords({}), "a"),
        Table.Sort(t, "a"),
        Table.Sort(t, "missing")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
