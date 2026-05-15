// Table.FromRows basic + with column types.
let r = try {
        Table.FromRows({{1, "a"}, {2, "b"}}, {"k", "v"}),
        Table.FromRows({}, {"k", "v"}),
        Table.FromRows({{1, "a"}}, {"k", "v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
