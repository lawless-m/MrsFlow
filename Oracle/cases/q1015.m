// Table.TransformColumnTypes multi-column.
let t = Table.FromRecords({
        [a="1", b="2.5"],
        [a="3", b="4.0"]
    }) in
let r = try {
        Table.TransformColumnTypes(t, {{"a", type number}, {"b", type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
