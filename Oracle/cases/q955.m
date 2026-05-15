// Table.AddColumn basic — no type annotation.
let t = Table.FromRecords({[a=1, b=2], [a=3, b=4]}) in
let r = try {
        Table.AddColumn(t, "c", each [a] + [b]),
        Table.AddColumn(t, "c", each [a] * [b]),
        Table.AddColumn(t, "c", each Text.From([a]))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
