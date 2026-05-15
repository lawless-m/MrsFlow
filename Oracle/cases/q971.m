// Table.ExpandListColumn basic + edge cases.
let t = Table.FromRecords({
        [k=1, lst={10, 20}],
        [k=2, lst={30}],
        [k=3, lst={}]
    }) in
let r = try {
        Table.ExpandListColumn(t, "lst")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
