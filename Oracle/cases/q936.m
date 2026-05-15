// Table.Group with GroupKind.Global (default) — full re-grouping.
// GroupKind.Local — only adjacent matching keys.
let t = Table.FromRecords({
        [k="a", v=1],
        [k="a", v=2],
        [k="b", v=3],
        [k="a", v=4]
    }) in
let r = try {
        // Global: 2 groups (a=[1,2,4], b=[3])
        Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}}, GroupKind.Global),
        // Local: 3 groups (a=[1,2], b=[3], a=[4])
        Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}}, GroupKind.Local)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
