// Table.Sort with explicit Order.Descending.
let t = Table.FromRecords({
        [name="b", v=2],
        [name="a", v=3],
        [name="c", v=1]
    }) in
let r = try {
        Table.Sort(t, {"v", Order.Descending}),
        Table.Sort(t, {"name", Order.Descending}),
        Table.Sort(t, {"v", Order.Ascending})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
