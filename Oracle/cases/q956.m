// Table.AddColumn with type ascription — basic types.
let t = Table.FromRecords({[a=1], [a=2], [a=3]}) in
let r = try {
        Table.AddColumn(t, "doubled",  each [a] * 2, type number),
        Table.AddColumn(t, "text",     each Text.From([a]), type text),
        Table.AddColumn(t, "isOdd",    each Number.Mod([a], 2) = 1, type logical),
        Table.AddColumn(t, "now",      each #date(2026, 1, 1), type date)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
