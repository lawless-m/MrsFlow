// Table.SelectRows null arg / non-function predicate.
let t = Table.FromRecords({[n=1]}) in
let r = try {
        Table.SelectRows(t, null),
        Table.SelectRows(t, "string-not-function"),
        Table.SelectRows(null, each true)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
