// Table.Pivot basic — pivot column to columns.
let t = Table.FromRecords({
        [k="A", attr="x", val=1],
        [k="A", attr="y", val=2],
        [k="B", attr="x", val=3],
        [k="B", attr="y", val=4]
    }) in
let r = try {
        Table.Pivot(t, {"x", "y"}, "attr", "val")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
