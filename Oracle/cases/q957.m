// Table.AddColumn type mismatch — type number annotation but text values.
// Does PQ coerce, refuse, or accept the mismatch silently?
let t = Table.FromRecords({[a=1]}) in
let r = try {
        Table.AddColumn(t, "c", each "not a number", type number)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
