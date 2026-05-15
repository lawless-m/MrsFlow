// Table.Group with null key values.
let t = Table.FromRecords({
        [k=null, v=1],
        [k="a", v=2],
        [k=null, v=3],
        [k="a", v=4]
    }) in
let r = try {
        Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
