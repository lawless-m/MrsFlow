// Pivot ∘ Unpivot round-trip.
let t = Table.FromRecords({
        [k="A", x=1, y=2],
        [k="B", x=3, y=4]
    }) in
let unp = Table.Unpivot(t, {"x", "y"}, "attr", "val") in
let r = try {
        Table.Pivot(unp, {"x", "y"}, "attr", "val")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
