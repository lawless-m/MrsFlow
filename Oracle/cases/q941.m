// Table.Sort single column default ascending.
let t = Table.FromRecords({
        [name="b", v=2],
        [name="a", v=3],
        [name="c", v=1]
    }) in
let r = try {
        Table.Sort(t, "name"),
        Table.Sort(t, "v")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
