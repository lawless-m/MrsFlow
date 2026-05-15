// Table.TransformColumnTypes round-trip: number → text → number.
let t = Table.FromRecords({[v=1.5], [v=2.0], [v=-3.14]}) in
let asText = Table.TransformColumnTypes(t, {{"v", type text}}) in
let r = try {
        Table.TransformColumnTypes(asText, {{"v", type number}}) = t
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
