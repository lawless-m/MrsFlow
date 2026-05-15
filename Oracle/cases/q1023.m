// Table.FromRows with mismatched row widths.
let r = try {
        Table.FromRows({{1, 2}, {3}}, {"a", "b"}),
        Table.FromRows({{1, 2, 3}}, {"a", "b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
