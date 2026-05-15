// Table.Sort stability — equal keys preserve input order.
let t = Table.FromRecords({
        [k=1, tag="A"],
        [k=2, tag="B"],
        [k=1, tag="C"],
        [k=2, tag="D"],
        [k=1, tag="E"]
    }) in
let r = try {
        Table.Sort(t, "k")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
