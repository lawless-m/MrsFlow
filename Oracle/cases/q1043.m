// Table.Combine with type-widening (number/text in same column).
let a = Table.FromRecords({[v=1], [v=2]}) in
let b = Table.FromRecords({[v="text"]}) in
let r = try {
        Table.Combine({a, b})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
