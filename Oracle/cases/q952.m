// Table.SelectRows with predicate returning non-logical / non-null.
let t = Table.FromRecords({[n=1], [n=2]}) in
let r = try {
        Table.SelectRows(t, each [n]),
        Table.SelectRows(t, each "yes")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
