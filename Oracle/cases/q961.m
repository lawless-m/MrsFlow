// Table.AddColumn with cell errors — does PQ tag the row or fail?
let t = Table.FromRecords({[a=1], [a=0], [a=2]}) in
let r = try {
        // 1/0 → +Inf (no error); 1/[a] when a=0 → +Inf too.
        Table.AddColumn(t, "div", each 1 / [a])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
