// Table.TransformColumnTypes with locale — de-DE uses comma decimal.
let t = Table.FromRecords({[v="1,5"], [v="2,75"], [v="-3,14"]}) in
let r = try {
        Table.TransformColumnTypes(t, {{"v", type number}}, "de-DE"),
        // Same input parsed with en-US would fail (comma isn't decimal).
        Table.TransformColumnTypes(t, {{"v", type number}}, "en-US")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
