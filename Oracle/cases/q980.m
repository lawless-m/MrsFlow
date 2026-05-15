// Table.Unpivot with null cells — does PQ drop or keep?
let t = Table.FromRecords({
        [k="A", x=1, y=null],
        [k="B", x=null, y=4]
    }) in
let r = try {
        Table.Unpivot(t, {"x", "y"}, "attr", "val")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
