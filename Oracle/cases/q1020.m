// Table.FromList — single-column table from a list.
let r = try {
        Table.FromList({1, 2, 3}, null, {"v"}),
        Table.FromList({"a", "b"}, null, {"v"}),
        Table.FromList({}, null, {"v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
