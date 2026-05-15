// Table.FillDown multi-column.
let t = Table.FromRecords({
        [k=1, a="A1", b="B1"],
        [k=2, a=null, b=null],
        [k=3, a=null, b="B3"],
        [k=4, a="A4", b=null]
    }) in
let r = try {
        Table.FillDown(t, {"a", "b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
