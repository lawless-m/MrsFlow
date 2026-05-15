// Table.Group with multiple aggregators per group.
let t = Table.FromRecords({
        [k="a", v=10],
        [k="b", v=20],
        [k="a", v=30],
        [k="b", v=40]
    }) in
let r = try {
        Table.Group(t, "k", {
            {"sum",   each List.Sum([v]),     type number},
            {"avg",   each List.Average([v]), type number},
            {"count", each Table.RowCount(_), type number},
            {"items", each [v],               type list}
        })
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
