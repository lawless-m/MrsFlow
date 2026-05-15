// Table.Group: error rows participate (or not?) in aggregation.
let t = Table.FromRecords({[g="A", v=1], [g="A", v=2], [g="B", v=3], [g="A", v=4]}) in
let t2 = Table.AddColumn(t, "v2", each if [v] = 2 then error "bad" else [v]) in
let r = try {
        Table.Group(t, {"g"}, {{"sum", each List.Sum([v]), Int64.Type}}),
        // After error injected, replace then group
        Table.Group(
            Table.ReplaceErrorValues(t2, {{"v2", 0}}),
            {"g"},
            {{"sum2", each List.Sum([v2]), Int64.Type}}
        )
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
