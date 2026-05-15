// Table.RemoveRowsWithErrors basic — no actual errors, table unchanged.
let t = Table.FromRecords({
        [k=1, v="a"],
        [k=2, v="b"],
        [k=3, v="c"]
    }) in
let r = try {
        Table.RemoveRowsWithErrors(t) = t
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
