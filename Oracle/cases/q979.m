// Table.Unpivot basic — wide → long.
let t = Table.FromRecords({
        [k="A", x=1, y=2],
        [k="B", x=3, y=4]
    }) in
let r = try {
        Table.Unpivot(t, {"x", "y"}, "attr", "val"),
        Table.Unpivot(t, {"x"}, "attr", "val")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
