// FillDown ∘ FillUp identity on already-filled column.
let t = Table.FromRecords({
        [v="A"],
        [v="B"],
        [v="C"]
    }) in
let r = try {
        Table.FillDown(t, {"v"}) = t,
        Table.FillUp(t, {"v"}) = t
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
