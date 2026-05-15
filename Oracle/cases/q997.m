// Table.FillDown basic — propagate non-null down into nulls.
let t = Table.FromRecords({
        [k=1, v="A"],
        [k=2, v=null],
        [k=3, v=null],
        [k=4, v="B"],
        [k=5, v=null]
    }) in
let r = try {
        Table.FillDown(t, {"v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
