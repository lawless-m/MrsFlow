// Table.Group basic — single key, single aggregator.
let t = Table.FromRecords({
        [k="a", v=1],
        [k="b", v=2],
        [k="a", v=3],
        [k="b", v=4]
    }) in
let r = try {
        Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}}),
        Table.Group(t, "k", {{"count", each Table.RowCount(_), type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
