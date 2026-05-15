// Table.FillDown where column starts with null — first nulls stay.
let t = Table.FromRecords({
        [k=1, v=null],
        [k=2, v=null],
        [k=3, v="A"],
        [k=4, v=null]
    }) in
let r = try {
        Table.FillDown(t, {"v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
