// Table.FromColumns with mismatched column lengths.
let r = try {
        Table.FromColumns({{1, 2, 3}, {"a", "b"}}, {"k", "v"}),
        Table.FromColumns({{1, 2}, {"a", "b", "c"}}, {"k", "v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
