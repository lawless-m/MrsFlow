// Table.Distinct with null cells — null dedupes against null.
let t = Table.FromRecords({
        [k=1, v=null],
        [k=2, v="A"],
        [k=3, v=null],
        [k=4, v="A"]
    }) in
let r = try {
        Table.Distinct(t, "v")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
