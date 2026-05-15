// Table.Combine empty list / single table / empty tables.
let t = Table.FromRecords({[k=1]}) in
let r = try {
        Table.Combine({}),
        Table.Combine({t}),
        Table.Combine({Table.FromRecords({}), Table.FromRecords({})}),
        Table.Combine({t, Table.FromRecords({})})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
