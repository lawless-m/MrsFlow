// Table.SelectRows basic — boolean predicate.
let t = Table.FromRecords({
        [n=1, v=10],
        [n=2, v=20],
        [n=3, v=30]
    }) in
let r = try {
        Table.SelectRows(t, each [n] > 1),
        Table.SelectRows(t, each [v] >= 20),
        Table.SelectRows(t, each false),
        Table.SelectRows(t, each true)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
