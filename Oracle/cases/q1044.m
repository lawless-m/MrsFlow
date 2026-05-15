// Table.Combine 3-way with progressively wider schemas.
let a = Table.FromRecords({[a=1]}) in
let b = Table.FromRecords({[a=2, b=20]}) in
let c = Table.FromRecords({[a=3, b=30, c="three"]}) in
let r = try {
        Table.Combine({a, b, c})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
