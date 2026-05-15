// Table.SelectRows with explicit null predicate result.
let t = Table.FromRecords({
        [n=1],
        [n=2],
        [n=3]
    }) in
let r = try {
        Table.SelectRows(t, each null),
        Table.SelectRows(t, each if [n] = 2 then null else [n] > 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
