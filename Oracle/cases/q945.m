// Table.Sort with nulls in sort column — null sorts first/last?
let t = Table.FromRecords({
        [v=2],
        [v=null],
        [v=1],
        [v=null],
        [v=3]
    }) in
let r = try {
        Table.Sort(t, "v"),
        Table.Sort(t, {"v", Order.Descending})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
