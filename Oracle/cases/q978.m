// Table.Pivot with null pivot column value.
let t = Table.FromRecords({
        [k="A", attr=null, val=1],
        [k="A", attr="x", val=2],
        [k="B", attr=null, val=3]
    }) in
let r = try {
        Table.Pivot(t, {"x"}, "attr", "val")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
