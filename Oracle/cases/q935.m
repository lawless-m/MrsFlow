// Table.Group multi-key.
let t = Table.FromRecords({
        [a=1, b="x", v=10],
        [a=1, b="y", v=20],
        [a=1, b="x", v=30],
        [a=2, b="x", v=40]
    }) in
let r = try {
        Table.Group(t, {"a", "b"}, {{"sum", each List.Sum([v]), type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
