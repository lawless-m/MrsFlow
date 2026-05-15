// Table.AddColumn collision — adding a column that already exists.
let t = Table.FromRecords({[a=1, b=2]}) in
let r = try {
        Table.AddColumn(t, "a", each 99)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
