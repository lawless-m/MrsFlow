// Table.Distinct with user lambda comparer — Phase 1 confirmed PQ rejects.
let t = Table.FromRecords({[v="a"], [v="A"]}) in
let r = try {
        Table.Distinct(t, {"v", (x, y) => Text.Lower(x) = Text.Lower(y)})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
