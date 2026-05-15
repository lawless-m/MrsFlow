// Table.Combine totally disjoint columns.
let a = Table.FromRecords({[a=1, b=2]}) in
let b = Table.FromRecords({[c=3, d=4]}) in
let r = try {
        Table.Combine({a, b})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
