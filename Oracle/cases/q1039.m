// Table.Combine same-schema — straight concatenation.
let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"]}) in
let b = Table.FromRecords({[k=3, v="b3"], [k=4, v="b4"]}) in
let r = try {
        Table.Combine({a, b})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
