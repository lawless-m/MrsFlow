// Table.Distinct empty / single / all-same rows.
let r = try {
        Table.Distinct(Table.FromRecords({})),
        Table.Distinct(Table.FromRecords({[k=1, v="A"]})),
        Table.Distinct(Table.FromRecords({
            [k=1, v="A"],
            [k=1, v="A"],
            [k=1, v="A"]
        }))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
