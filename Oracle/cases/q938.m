// Table.Group with empty input table.
let empty = Table.FromRecords({}) in
let r = try {
        Table.Group(empty, "k", {{"sum", each 0, type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
