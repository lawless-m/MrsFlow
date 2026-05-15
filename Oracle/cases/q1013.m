// Table.TransformColumnTypes with locale — fr-FR (space or NBSP thousands, comma decimal).
let t = Table.FromRecords({[v="1234,5"], [v="1234,56"]}) in
let r = try {
        Table.TransformColumnTypes(t, {{"v", type number}}, "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
