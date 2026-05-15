// Table.Combine extra column in b — null-fill into a's missing column.
let a = Table.FromRecords({[k=1, v="a1"]}) in
let b = Table.FromRecords({[k=2, v="b2", extra=100]}) in
let r = try {
        Table.Combine({a, b})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
