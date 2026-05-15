// Table.Combine column ordering — does PQ preserve insertion order of new
// columns as they appear in successive tables?
let a = Table.FromRecords({[x=1, y=2]}) in
let b = Table.FromRecords({[y=20, z=30]}) in
let r = try {
        Table.Combine({a, b}),
        Table.Combine({b, a})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
