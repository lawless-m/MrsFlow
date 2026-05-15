// Table.Sort multi-column — mixed ascending/descending.
let t = Table.FromRecords({
        [g=1, v=30],
        [g=2, v=10],
        [g=1, v=20],
        [g=2, v=40]
    }) in
let r = try {
        Table.Sort(t, {{"g", Order.Ascending}, {"v", Order.Descending}}),
        Table.Sort(t, {{"g", Order.Descending}, {"v", Order.Ascending}}),
        Table.Sort(t, {"g", "v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
