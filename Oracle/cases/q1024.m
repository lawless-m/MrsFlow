// Round-trip identity: Records ↔ Rows ↔ Records.
let original = Table.FromRecords({[a=1, b="x"], [a=2, b="y"]}) in
let rows = Table.ToRows(original) in
let r = try {
        Table.FromRows(rows, {"a", "b"}) = original
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
