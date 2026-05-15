// Table.UnpivotOtherColumns — invert column selection.
let t = Table.FromRecords({
        [k="A", x=1, y=2, z=3],
        [k="B", x=4, y=5, z=6]
    }) in
let r = try {
        Table.UnpivotOtherColumns(t, {"k"}, "attr", "val")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
