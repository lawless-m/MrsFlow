// Table.Group with key column not in table.
let t = Table.FromRecords({[k="a", v=1]}) in
let r = try {
        Table.Group(t, "missing", {{"sum", each List.Sum([v]), type number}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
