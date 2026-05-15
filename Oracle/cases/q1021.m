// Table.FromColumns basic — list-of-columns.
let r = try {
        Table.FromColumns({{1, 2, 3}, {"a", "b", "c"}}, {"k", "v"}),
        Table.FromColumns({{1, 2}, {"a", "b"}, {true, false}}, {"k", "v", "f"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
