// Table.SelectRows compound predicate / multi-column.
let t = Table.FromRecords({
        [a=1, b=10],
        [a=2, b=20],
        [a=1, b=30],
        [a=2, b=40]
    }) in
let r = try {
        Table.SelectRows(t, each [a] = 1 and [b] > 15),
        Table.SelectRows(t, each [a] = 1 or [b] > 30),
        Table.SelectRows(t, each not ([a] = 1))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
