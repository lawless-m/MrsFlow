// Table.TransformColumnTypes basic — text → number.
let t = Table.FromRecords({[v="1"], [v="2.5"], [v="-3"]}) in
let r = try {
        Table.TransformColumnTypes(t, {{"v", type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
