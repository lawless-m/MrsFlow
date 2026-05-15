// Table.NestedJoin with error cells in the inner table: error rows
// survive through the nested cell, then ExpandTable propagates.
let l = Table.FromRecords({[id=1, name="A"], [id=2, name="B"]}) in
let r0 = Table.FromRecords({[id=1, v=10], [id=2, v=20]}) in
let r1 = Table.AddColumn(r0, "v2", each if [id] = 1 then error "bad" else [v]) in
let r2 = Table.ReplaceErrorValues(r1, {{"v2", -1}}) in
let r = try {
        Table.NestedJoin(l, {"id"}, r2, {"id"}, "joined", JoinKind.Inner),
        Table.ExpandTableColumn(
            Table.NestedJoin(l, {"id"}, r2, {"id"}, "joined", JoinKind.Inner),
            "joined", {"v2"}
        )
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
