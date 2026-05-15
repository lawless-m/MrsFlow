// Inside a table cell: Number.ToText consistent with Json.FromValue.
let t = Table.FromRecords({[v=999999999999999], [v=1000000000000000], [v=9007199254740992]}) in
let r = try {
        t,
        Table.AddColumn(t, "s", each Text.From([v]))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
