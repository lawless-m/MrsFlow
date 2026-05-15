// Table.TransformColumnTypes date with locale — de-DE dd.MM.yyyy.
let t = Table.FromRecords({[d="15.06.2026"], [d="01.01.2026"]}) in
let r = try {
        Table.TransformColumnTypes(t, {{"d", type date}}, "de-DE")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
