// Table.SelectRows with predicate returning null — does PQ exclude
// (treat null as false) or include?
let t = Table.FromRecords({
        [n=1, v=10],
        [n=2, v=null],
        [n=3, v=30]
    }) in
let r = try {
        // Predicate that returns null when v is null.
        Table.SelectRows(t, each [v] > 15)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
